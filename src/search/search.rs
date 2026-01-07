use std::cmp::Reverse;

use arrayvec::ArrayVec;
use icarus_board::{board::TerminalState, r#move::Move};

use crate::{pesto::eval, position::Position, score::Score, search::searcher::ThreadCtx};

#[derive(Clone, Copy, Debug)]
pub struct ScoredMove(pub Move, pub i16);

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

    let mut moves: ArrayVec<ScoredMove, 218> =
        pos.board().gen_all_moves_to_mapped(|mv| ScoredMove(mv, 0));

    for mv in &mut moves {
        let victim = 8 * mv.0.captures(pos.board()).map_or(0, |p| p.idx() + 1) as i16;
        let attacker = mv.0.piece_type(pos.board()) as i16;
        mv.1 = victim - attacker * i16::from(victim != 0);
    }

    moves.sort_unstable_by_key(|mv| Reverse(mv.1));

    let mut max = -Score::INFINITE;

    for (i, ScoredMove(mv, _)) in moves.into_iter().enumerate() {
        pos.make_move(mv);
        let score = -search(pos, depth - 1, ply + 1, -beta, -alpha, thread);
        pos.unmake_move();
        if thread.abort_now {
            return Score::ZERO;
        }

        if score > max {
            max = score;

            if i == 0 || score > alpha {
                alpha = score;

                let [parent, child] = thread
                    .search_stack
                    .get_disjoint_mut([ply as usize, ply as usize + 1])
                    .unwrap();

                parent.pv = ArrayVec::from_iter([mv]);
                parent.pv.extend(child.pv.iter().copied());
            }
        }
        if score >= beta {
            break;
        }
    }

    max
}
