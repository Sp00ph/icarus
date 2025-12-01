use icarus_board::board::Board;
use rustyline::{Config, Editor, error::ReadlineError, history::MemHistory};

use crate::uci::UciCommand;

mod uci;

fn main() -> Result<(), rootcause::Report> {
    let mut editor = Editor::<(), MemHistory>::with_history(
        Config::builder()
            .auto_add_history(true)
            .enable_signals(true)
            .build(),
        MemHistory::new(),
    )?;

    let mut board = Board::start_pos();
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

        let command = UciCommand::parse(line, &board, true);
        println!("{command:?}");

        match command {
            Ok(UciCommand::Position { board: new, moves }) => {
                board = new;
                for mv in moves {
                    board.make_move(mv);
                }
            }
            Ok(UciCommand::Display) => {
                board.print(true);
            }
            _ => {}
        }
    }

    Ok(())
}
