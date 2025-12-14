use std::time::{Duration, Instant};

use icarus_board::{board::Board, movegen::Abort, perft::perft};
use icarus_common::r#move::Move;
use rustyline::{Config, Editor, error::ReadlineError, history::MemHistory};

use crate::uci::UciCommand;

pub struct Engine {
    board: Board,
    chess960: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::start_pos(),
            chess960: false,
        }
    }

    pub fn run(&mut self) -> Result<(), rootcause::Report> {
        let mut editor = Editor::<(), MemHistory>::with_history(
            Config::builder()
                .auto_add_history(true)
                .enable_signals(true)
                .build(),
            MemHistory::new(),
        )?;

        loop {
            let line = match editor.readline("") {
                Ok(line) => line,
                Err(ReadlineError::Eof) => break,
                Err(e) => return Err(e.into()),
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let command = match UciCommand::parse(line, &self.board, self.chess960) {
                Ok(command) => command,
                Err(e) => {
                    eprintln!("info string {e}");
                    continue;
                }
            };

            match command {
                UciCommand::Uci => self.uci(),
                UciCommand::NewGame => self.newgame(),
                UciCommand::IsReady => self.isready(),
                UciCommand::SetOption { name, value } => self.setoption(name, value),
                UciCommand::Position { board, moves } => self.position(board, moves),
                UciCommand::Go(_search_limits) => todo!(),
                UciCommand::Eval => todo!(),
                UciCommand::Display => self.display(),
                UciCommand::Bench {
                    depth: _,
                    threads: _,
                    hash: _,
                } => todo!(),
                UciCommand::Perft(depth) => self.perft(depth),
                UciCommand::SplitPerft(depth) => self.splitperft(depth),
                UciCommand::Stop => todo!(),
                UciCommand::Quit => todo!(),
            }
        }

        Ok(())
    }

    fn uci(&self) {
        println!("id name Icarus 0.0.0-dev");
        println!("id author Sp00ph");
        println!("option UCI_Chess960 type check default false");
        println!("uciok");
    }

    fn newgame(&mut self) {
        self.board = Board::start_pos();
    }

    fn isready(&self) {
        println!("readyok");
    }

    fn setoption(&mut self, name: String, value: String) {
        match name.as_str() {
            "UCI_Chess960" => {
                let Ok(val) = value.parse::<bool>() else {
                    println!("info string Unknown value {value}");
                    return;
                };
                self.chess960 = val;
                println!("info string Set Chess960 to {val}");
            }
            _ => println!("info string Unsupported option {name}"),
        }
    }

    fn position(&mut self, board: Board, moves: Vec<Move>) {
        self.board = board;
        for mv in moves {
            self.board.make_move(mv);
        }
    }

    fn display(&self) {
        self.board.print(self.chess960);
    }

    fn perft(&self, depth: u8) {
        let t = Instant::now();
        let n = perft(&self.board, depth);
        let d = t.elapsed();
        let mnps = (n as f64) / d.as_secs_f64() / 1e6;
        println!("Total: {n}");
        println!("Took {d:.2?} ({mnps:.2}Mnps)\n");
    }

    fn splitperft(&self, depth: u8) {
        if depth == 0 {
            println!("No!");
            return;
        }

        let mut moves = vec![];
        self.board.gen_moves(|mv| {
            moves.extend(mv);
            Abort::No
        });

        let mut d = Duration::ZERO;
        let mut total = 0u64;
        for mv in moves {
            let mut board = self.board;
            board.make_move(mv);
            let t = Instant::now();
            let n = perft(&board, depth - 1);
            d += t.elapsed();
            total += n;
            println!("{}: {n}", mv.display(self.chess960));
        }

        let mnps = (total as f64) / d.as_secs_f64() / 1e6;
        println!("\nTotal: {total}");
        println!("Took {d:.2?} ({mnps:.2}Mnps)\n");
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
