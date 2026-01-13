use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use icarus_board::{board::Board, r#move::Move, movegen::Abort, perft::perft};
use rustyline::{Config, Editor, error::ReadlineError, history::MemHistory};

use crate::{
    bench::DEFAULT_BENCH_DEPTH,
    pesto::eval,
    position::Position,
    search::{searcher::Searcher, time_manager::DEFAULT_MOVE_OVERHEAD},
    uci::{SearchLimit, UciCommand},
    util::atomic_instant::EPOCH,
};

pub struct Engine {
    position: Position,
    chess960: bool,
    searcher: Searcher,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            position: Position::new(Board::start_pos()),
            chess960: false,
            searcher: Searcher::default(),
        }
    }

    pub fn run(&mut self) -> Result<(), rootcause::Report> {
        // Initialize the epoch used for `AtomicInstant`.
        LazyLock::force(&EPOCH);

        let argv: Vec<String> = std::env::args().skip(1).collect();

        if argv == ["bench"] {
            self.bench(DEFAULT_BENCH_DEPTH, true);
            return Ok(());
        }

        #[cfg(feature = "test-islegal")]
        if argv == ["test_islegal"] {
            crate::test_islegal::test_islegal();
        }

        let mut editor = Editor::<(), MemHistory>::with_history(
            Config::builder().auto_add_history(true).build(),
            MemHistory::new(),
        )?;

        let mut argv = argv.into_iter();
        loop {
            let line = match argv.next() {
                Some(line) => line,
                None => match editor.readline("") {
                    Ok(line) => line,
                    Err(ReadlineError::Eof) => break,
                    Err(e) => return Err(e.into()),
                },
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let command = match UciCommand::parse(line, self.position.board(), self.chess960) {
                Ok(command) => command,
                Err(e) => {
                    eprintln!("info string {e}");
                    continue;
                }
            };

            if self.handle_cmd(command) == Abort::Yes {
                break;
            }
        }

        Ok(())
    }

    fn handle_cmd(&mut self, command: UciCommand) -> Abort {
        match command {
            UciCommand::Uci => self.uci(),
            UciCommand::NewGame => self.newgame(),
            UciCommand::IsReady => self.isready(),
            UciCommand::SetOption { name, value } => self.setoption(name, value),
            UciCommand::Position { board, moves } => self.position(board, moves),
            UciCommand::Go(search_limits) => self.go(search_limits),
            UciCommand::Eval => self.eval(),
            UciCommand::Display => self.display(),
            UciCommand::Bench { depth, .. } => self.bench(depth, false),
            UciCommand::Perft { depth, bulk } => self.perft(depth, bulk),
            UciCommand::SplitPerft { depth, bulk } => self.splitperft(depth, bulk),
            UciCommand::Stop => self.stop(),
            UciCommand::Quit => {
                self.quit();
                return Abort::Yes;
            }
            UciCommand::Wait => self.wait(true),
        }

        Abort::No
    }

    fn uci(&self) {
        let version = env!("CARGO_PKG_VERSION");
        println!("id name Icarus {version}-dev");
        println!("id author Sp00ph");
        println!("option name UCI_Chess960 type check default false");
        println!(
            "option name MoveOverhead type spin default {} min 0 max {}",
            DEFAULT_MOVE_OVERHEAD,
            u16::MAX
        );
        println!("uciok");
    }

    fn newgame(&mut self) {
        self.position = Position::new(Board::start_pos());
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
            "MoveOverhead" => {
                let Ok(val) = value.parse::<u16>() else {
                    println!("info string Unknown value {value}");
                    return;
                };
                self.searcher.global_ctx.time_manager.set_move_overhead(val);
                println!("info string Set move overhead to {val}");
            }
            _ => println!("info string Unsupported option {name}"),
        }
    }

    fn position(&mut self, board: Board, moves: Vec<Move>) {
        self.position = Position::new(board);
        for mv in moves {
            self.position.make_move(mv);
        }
    }

    fn display(&self) {
        self.position.board().print(self.chess960);
    }

    fn perft(&self, depth: u8, bulk: bool) {
        let board = *self.position.board();
        std::thread::spawn(move || {
            let t = Instant::now();
            let n = if bulk {
                perft::<true>(&board, depth)
            } else {
                perft::<false>(&board, depth)
            };
            let d = t.elapsed();
            let mnps = (n as f64) / d.as_secs_f64() / 1e6;
            println!("Total: {n}");
            println!("Took {d:.2?} ({mnps:.2}Mnps)\n");
        });
    }

    fn splitperft(&self, depth: u8, bulk: bool) {
        if depth == 0 {
            println!("No!");
            return;
        }

        let board = *self.position.board();
        let chess960 = self.chess960;

        std::thread::spawn(move || {
            let moves: Vec<Move> = board.gen_all_moves_to();

            let mut d = Duration::ZERO;
            let mut total = 0u64;
            for mv in moves {
                let mut board = board;
                board.make_move(mv);
                let t = Instant::now();
                let n = if bulk {
                    perft::<true>(&board, depth - 1)
                } else {
                    perft::<false>(&board, depth - 1)
                };
                d += t.elapsed();
                total += n;
                println!("{}: {n}", mv.display(chess960));
            }

            let mnps = (total as f64) / d.as_secs_f64() / 1e6;
            println!("\nTotal: {total}");
            println!("Took {d:.2?} ({mnps:.2}Mnps)\n");
        });
    }

    fn go(&mut self, search_limits: Vec<SearchLimit>) {
        if self.searcher.is_running() {
            println!("info string already searching");
            return;
        }
        self.searcher
            .search(self.position.clone(), search_limits, self.chess960, true);
    }

    fn stop(&mut self) {
        if self.searcher.is_running() {
            self.searcher.stop();
            self.searcher.wait();
            println!("info string stopped search");
        } else {
            println!("info string search isn't running")
        }
    }

    fn quit(&mut self) {
        self.searcher.quit();
    }

    fn wait(&self, print: bool) {
        if !self.searcher.is_running() {
            if print {
                println!("info string search isn't running");
            }
        } else {
            if print {
                println!("info string waiting for search to end...");
            }
            self.searcher.wait();
            if print {
                println!("info string searcher stopped");
            }
        }
    }

    fn eval(&self) {
        let score = eval(self.position.board());
        println!("Static eval: {score:#}");
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
