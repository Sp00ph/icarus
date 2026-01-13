use std::{
    num::ParseIntError,
    str::{FromStr, ParseBoolError},
};

use icarus_board::{board::Board, r#move::Move};

use crate::bench::DEFAULT_BENCH_DEPTH;

#[derive(Debug)]
pub enum UciCommand {
    Uci,
    NewGame,
    IsReady,
    SetOption { name: String, value: String },
    Position { board: Board, moves: Vec<Move> },
    Go(Vec<SearchLimit>),
    Eval,
    Display,
    Bench { depth: u8, threads: u16, hash: u32 },
    Perft { depth: u8, bulk: bool },
    SplitPerft { depth: u8, bulk: bool },
    Stop,
    Quit,
    Wait,
}

#[derive(Debug, Clone)]
pub enum SearchLimit {
    SearchMoves(Vec<Move>),
    WhiteTime(u64),
    BlackTime(u64),
    WhiteInc(u64),
    BlackInc(u64),
    MoveTime(u64),
    Depth(u16),
    Nodes(u64),
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum UciParseError {
    #[error("Unknown command: `{0}`")]
    UnknownCommand(String),
    #[error("No command found")]
    MissingCommand,
    #[error("Missing `name` token on `setoption` command")]
    MissingOptionNameToken,
    #[error("Missing option name on `setoption` command")]
    MissingOptionName,
    #[error("Missing `value` token on `setoption` command")]
    MissingOptionValueToken,
    #[error("Missing option value on `setoption` command")]
    MissingOptionValue,
    #[error("Missing `fen` or `startpos` on `position` command")]
    MissingPositionType,
    #[error("Invalid FEN `{0}`")]
    InvalidFen(String),
    #[error("Missing `moves` token on `position` command")]
    MissingPositionMovesToken,
    #[error("Invalid or illegal move `{0}`")]
    InvalidMove(String),
    #[error("Unknown search limit: {0}")]
    UnknownLimit(String),
    #[error("Missing value for limit `{0}`")]
    MissingLimitValue(String),
    #[error("Error parsing integer: {0}")]
    InvalidInt(#[from] ParseIntError),
    #[error("Error parsing boolean: {0}")]
    InvalidBool(#[from] ParseBoolError),
}

impl UciCommand {
    pub fn parse(s: &str, board: &Board, chess960: bool) -> Result<Self, UciParseError> {
        use UciCommand::*;
        use UciParseError::*;

        let mut reader = s.trim().split_ascii_whitespace();
        let cmd = reader.next().ok_or(MissingCommand)?;

        match cmd {
            "uci" => Ok(Uci),
            "isready" => Ok(IsReady),
            "ucinewgame" => Ok(NewGame),
            "eval" => Ok(Eval),
            "d" => Ok(Display),
            "stop" => Ok(Stop),
            "quit" | "q" => Ok(Quit),
            "wait" => Ok(Wait),
            "setoption" => {
                if reader.next() != Some("name") {
                    return Err(MissingOptionNameToken);
                }

                let name = reader.next().ok_or(MissingOptionName)?;
                let Some(value_token) = reader.next() else {
                    return Ok(SetOption {
                        name: name.into(),
                        value: "<empty>".into(),
                    });
                };
                if value_token != "value" {
                    return Err(MissingOptionValueToken);
                }

                let value = reader.next().ok_or(MissingOptionValue)?;
                Ok(SetOption {
                    name: name.into(),
                    value: value.into(),
                })
            }
            // option shorthand: so <name> <value>
            "so" => {
                let name = reader.next().ok_or(MissingOptionName)?;
                let value = reader.next().unwrap_or("<empty>");
                Ok(SetOption {
                    name: name.into(),
                    value: value.into(),
                })
            }
            // bench <depth> <threads> <hash>
            "bench" => {
                let depth = reader.next().map_or(Ok(DEFAULT_BENCH_DEPTH), str::parse)?;
                let threads = reader.next().unwrap_or("1").parse()?;
                let hash = reader.next().unwrap_or("16").parse()?;
                Ok(Bench {
                    depth,
                    threads,
                    hash,
                })
            }
            "perft" => Ok(Perft {
                depth: reader.next().unwrap_or("6").parse()?,
                bulk: reader.next().unwrap_or("true").parse()?,
            }),
            "splitperft" => Ok(SplitPerft {
                depth: reader.next().unwrap_or("6").parse()?,
                bulk: reader.next().unwrap_or("true").parse()?,
            }),
            "position" => {
                let startpos = match reader.next() {
                    Some("startpos") => Board::start_pos(),
                    Some("kiwipete") => Board::read_fen(
                        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                    )
                    .unwrap(),
                    Some("fen") => {
                        let mut fen = String::new();

                        // FEN consists of 6 parts (board, stm, castling rights, ep square, hmc, fmc)
                        for part in reader.by_ref().take(6) {
                            if !fen.is_empty() {
                                fen.push(' ');
                            }
                            fen.push_str(part);
                        }
                        Board::read_fen(&fen).ok_or(InvalidFen(fen))?
                    }
                    _ => return Err(MissingPositionType),
                };

                if reader.next().is_some_and(|token| token != "moves") {
                    return Err(MissingPositionMovesToken);
                }

                let mut current = startpos;
                let mut moves = vec![];
                for part in reader {
                    let mv = current
                        .parse_move(part, chess960)
                        .ok_or_else(|| InvalidMove(part.to_string()))?;

                    if !current.is_legal(mv) {
                        return Err(InvalidMove(part.to_string()));
                    }
                    moves.push(mv);
                    current.make_move(mv);
                }

                Ok(Position {
                    board: startpos,
                    moves,
                })
            }
            "go" => {
                use SearchLimit::*;

                let keywords = [
                    "searchmoves",
                    "wtime",
                    "btime",
                    "winc",
                    "binc",
                    "depth",
                    "nodes",
                    "movetime",
                    "infinite",
                ];

                let mut reader = reader.peekable();
                let mut limits = vec![];

                fn parse_int<'a, T: FromStr<Err = ParseIntError>>(
                    reader: &mut impl Iterator<Item = &'a str>,
                    part: &str,
                ) -> Result<T, UciParseError> {
                    Ok(reader
                        .next()
                        .ok_or_else(|| MissingLimitValue(part.into()))?
                        .parse()?)
                }

                while let Some(part) = reader.next() {
                    match part {
                        // infinite doesn't add any limits.
                        "infinite" => {}
                        "wtime" => limits
                            .push(WhiteTime(parse_int::<i64>(&mut reader, part)?.max(0) as u64)),
                        "btime" => limits
                            .push(BlackTime(parse_int::<i64>(&mut reader, part)?.max(0) as u64)),
                        "winc" => limits.push(WhiteInc(parse_int(&mut reader, part)?)),
                        "binc" => limits.push(BlackInc(parse_int(&mut reader, part)?)),
                        "depth" => limits.push(Depth(parse_int(&mut reader, part)?)),
                        "nodes" => limits.push(Nodes(parse_int(&mut reader, part)?)),
                        "movetime" => limits.push(MoveTime(parse_int(&mut reader, part)?)),
                        "searchmoves" => {
                            let mut moves = vec![];
                            while let Some(&token) = reader.peek()
                                && !keywords.contains(&token)
                            {
                                let mv = board
                                    .parse_move(token, chess960)
                                    .ok_or_else(|| InvalidMove(token.into()))?;
                                if !board.is_legal(mv) {
                                    return Err(InvalidMove(token.into()));
                                }
                                moves.push(mv);
                                // consume token
                                reader.next();
                            }

                            limits.push(SearchMoves(moves));
                        }
                        _ => return Err(UnknownLimit(part.into())),
                    }
                }

                Ok(Go(limits))
            }
            _ => Err(UnknownCommand(cmd.into())),
        }
    }
}
