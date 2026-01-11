use arrayvec::ArrayVec;
use icarus_board::board::TerminalState;

use crate::{
    pesto::eval,
    position::Position,
    score::Score,
    search::{move_picker::MovePicker, searcher::ThreadCtx},
};

pub fn search(
    pos: &mut Position,
    depth: i32,
    ply: u16,
    mut alpha: Score,
    beta: Score,
    thread: &mut ThreadCtx,
) -> Score {
    if ply != 0 && (thread.abort_now || (thread.global.time_manager.stop_search(&thread.nodes))) {
        thread.abort_now = true;
        return Score::ZERO;
    }

    thread.sel_depth = thread.sel_depth.max(ply);
    thread.nodes.inc();
    thread.search_stack[ply as usize].pv.clear();

    if let Some(terminal) = pos.board().terminal_state() {
        return match terminal {
            TerminalState::Checkmate(_) => Score::new_mated(ply),
            TerminalState::Draw => Score::ZERO,
        };
    }

    if ply > 0 && pos.repetition() {
        return Score::ZERO;
    }

    if depth <= 0 {
        return eval(pos.board());
    }

    let mut move_picker = MovePicker::default();

    let mut max = -Score::INFINITE;

    let mut moves_seen = 0;

    while let Some(mv) = move_picker.next(pos.board(), thread) {
        pos.make_move(mv);
        let score = -search(pos, depth - 1, ply + 1, -beta, -alpha, thread);
        pos.unmake_move();
        if thread.abort_now {
            return Score::ZERO;
        }

        if moves_seen == 0 || score > alpha {
            let [parent, child] = thread
                .search_stack
                .get_disjoint_mut([ply as usize, ply as usize + 1])
                .unwrap();

            parent.pv = ArrayVec::from_iter([mv]);
            parent.pv.extend(child.pv.iter().copied());
        }

        if score > max {
            max = score;
            if score > alpha {
                alpha = score;
            }
        }
        if score >= beta {
            break;
        }
        moves_seen += 1;
    }

    max
}
