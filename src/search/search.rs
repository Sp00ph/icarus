use arrayvec::ArrayVec;
use icarus_board::{board::TerminalState, r#move::Move};
use smallvec::SmallVec;

use crate::{
    pesto::eval,
    position::Position,
    score::Score,
    search::{
        lmr::get_lmr, move_picker::MovePicker, searcher::ThreadCtx, transposition_table::TTFlag,
    },
    util::MAX_PLY,
};

pub trait NodeType {
    const ROOT: bool;
    const PV: bool;

    type Next: NodeType;
}

pub struct Root;
pub struct PV;
pub struct NonPV;

impl NodeType for Root {
    const ROOT: bool = true;
    const PV: bool = true;
    type Next = PV;
}

impl NodeType for PV {
    const ROOT: bool = false;
    const PV: bool = true;
    type Next = PV;
}

impl NodeType for NonPV {
    const ROOT: bool = false;
    const PV: bool = false;
    type Next = NonPV;
}

pub fn search<Node: NodeType>(
    pos: &mut Position,
    depth: i16,
    ply: u16,
    mut alpha: Score,
    beta: Score,
    thread: &mut ThreadCtx,
) -> Score {
    if !Node::ROOT && (thread.abort_now || thread.global.time_manager.stop_search(&thread.nodes)) {
        thread.abort_now = true;
        return Score::ZERO;
    }

    thread.sel_depth = thread.sel_depth.max(ply);
    if Node::PV {
        thread.search_stack[ply as usize].pv.clear();
    }

    if let Some(terminal) = pos.board().terminal_state() {
        thread.nodes.inc();
        return match terminal {
            TerminalState::Checkmate(_) => Score::new_mated(ply),
            TerminalState::Draw => Score::ZERO,
        };
    }

    if !Node::ROOT && pos.repetition() {
        thread.nodes.inc();
        return Score::ZERO;
    }

    if depth <= 0 {
        return qsearch::<Node>(pos, ply, alpha, beta, thread);
    }

    if !Node::ROOT {
        thread.nodes.inc();
    }

    let tt_entry = thread.global.ttable.fetch(pos.board().hash(), ply);
    let tt_move = tt_entry.and_then(|e| e.mv);

    // TT cutoffs
    if !Node::PV
        && let Some(e) = tt_entry
        && e.depth as i16 >= depth
    {
        let score = e.score;
        match e.flags.tt_flag() {
            TTFlag::Exact => return score,
            TTFlag::Lower if score >= beta => return score,
            TTFlag::Upper if score <= alpha => return score,
            _ => {}
        }
    }

    let raw_eval = tt_entry
        .map(|e| e.eval)
        .unwrap_or_else(|| eval(pos.board()));
    let static_eval =
        Score::clamp_nomate(raw_eval.0.saturating_add(thread.history.corr(pos.board())));
    let in_check = pos.board().checkers().is_non_empty();

    if !Node::PV && !in_check {
        // RFP
        let rfp_depth = 6;
        let rfp_margin = 80;
        if depth < rfp_depth && static_eval - rfp_margin * depth >= beta {
            return static_eval;
        }

        // NMP
        let nmp_depth = 3;
        if depth >= nmp_depth && static_eval >= beta && pos.prev_move(1).is_some() {
            pos.make_null_move();
            let nmp_reduction = 3;
            let score = -search::<NonPV>(
                pos,
                depth - nmp_reduction,
                ply + 1,
                -beta,
                -beta + 1,
                thread,
            );
            pos.unmake_null_move();

            if thread.abort_now {
                return Score::ZERO;
            }

            if score >= beta {
                return beta;
            }
        }
    }

    let mut move_picker = MovePicker::new(tt_move, false, 0, false);
    let mut best_score = -Score::INFINITE;
    let mut moves_seen = 0;
    let mut best_move = None;
    let mut flag = TTFlag::Upper;

    // For quiet hist
    let mut quiets = SmallVec::<[Move; 64]>::new();

    while let Some(mv) = move_picker.next(pos, thread) {
        let is_tactic = pos.board().is_tactic(mv);
        let mut lmr = get_lmr(is_tactic, depth as u8, moves_seen);
        let mut score;

        'lmp_fp: {
            if !Node::ROOT && !best_score.is_loss() {
                // LMP
                let lmp_margin = 4096 + 1024 * (depth as u32).pow(2);

                if moves_seen as u32 * 1024 >= lmp_margin {
                    move_picker.skip_quiets();
                    break 'lmp_fp;
                }

                // FP
                let fp_depth = 8;
                let fp_base = 100;
                let fp_scale = 80;

                let fp_margin = fp_base + fp_scale * depth;
                if !Node::PV && depth <= fp_depth && !in_check && static_eval + fp_margin <= alpha {
                    move_picker.skip_quiets();
                }
            }
        }

        let new_depth = depth - 1;
        pos.make_move(mv);

        // PVS
        if moves_seen == 0 {
            score = -search::<Node::Next>(pos, new_depth, ply + 1, -beta, -alpha, thread);
        } else {
            if depth < 2 {
                lmr = 0;
            }
            let lmr_depth = (new_depth - lmr).max(1).min(new_depth);

            score = -search::<NonPV>(pos, lmr_depth, ply + 1, -alpha - 1, -alpha, thread);

            if lmr > 0 && score > alpha {
                score = -search::<NonPV>(pos, new_depth, ply + 1, -alpha - 1, -alpha, thread)
            }
            if Node::PV && score > alpha {
                score = -search::<PV>(pos, new_depth, ply + 1, -beta, -alpha, thread);
            }
        }

        pos.unmake_move();
        moves_seen += 1;
        if thread.abort_now {
            return Score::ZERO;
        }

        if Node::PV && (moves_seen == 1 || score > alpha) {
            let [parent, child] = thread
                .search_stack
                .get_disjoint_mut([ply as usize, ply as usize + 1])
                .unwrap();

            parent.pv = ArrayVec::from_iter([mv]);
            parent.pv.extend(child.pv.iter().copied());
        }

        if score > best_score {
            best_score = score;
            if score > alpha {
                alpha = score;
                best_move = Some(mv);
                flag = TTFlag::Exact;
            }
        }

        if score >= beta {
            flag = TTFlag::Lower;
            thread.history.update(pos.board(), mv, &quiets, depth);
            break;
        }

        if best_move != Some(mv) && !is_tactic {
            quiets.push(mv);
        }
    }

    thread.global.ttable.store(
        pos.board().hash(),
        depth as u8,
        ply,
        raw_eval,
        best_score,
        best_move,
        flag,
        true,
    );

    if !in_check
        && best_move.is_none_or(|mv| pos.board().is_quiet(mv))
        && match flag {
            TTFlag::Lower => best_score > static_eval,
            TTFlag::Upper => best_score < static_eval,
            _ => true,
        }
    {
        thread
            .history
            .update_corr(pos.board(), depth, best_score, static_eval);
    }

    best_score
}

pub fn qsearch<Node: NodeType>(
    pos: &mut Position,
    ply: u16,
    mut alpha: Score,
    beta: Score,
    thread: &mut ThreadCtx,
) -> Score {
    if thread.abort_now || thread.global.time_manager.stop_search(&thread.nodes) {
        thread.abort_now = true;
        return Score::ZERO;
    }

    thread.sel_depth = thread.sel_depth.max(ply);
    if Node::PV {
        thread.search_stack[ply as usize].pv.clear();
    }

    if let Some(terminal) = pos.board().terminal_state() {
        return match terminal {
            TerminalState::Checkmate(_) => Score::new_mated(ply),
            TerminalState::Draw => Score::ZERO,
        };
    }

    if pos.repetition() {
        return Score::ZERO;
    }

    if ply >= MAX_PLY {
        return eval(pos.board());
    }

    thread.sel_depth = thread.sel_depth.max(ply);
    thread.nodes.inc();

    let in_check = pos.board().checkers().is_non_empty();

    if !in_check {
        let eval = eval(pos.board());

        if eval >= beta {
            return eval;
        }

        if eval >= alpha {
            alpha = eval;
        }
    }

    let tt_entry = thread.global.ttable.fetch(pos.board().hash(), ply);

    // TT cutoffs
    if !Node::PV
        && let Some(e) = tt_entry
    {
        let score = e.score;
        match e.flags.tt_flag() {
            TTFlag::Exact => return score,
            TTFlag::Lower if score >= beta => return score,
            TTFlag::Upper if score <= alpha => return score,
            _ => {}
        }
    }

    let mut max = -Score::INFINITE;
    let mut moves_seen = 0;
    let mut move_picker = MovePicker::new(None, !in_check, 0, true);

    while let Some(mv) = move_picker.next(pos, thread) {
        pos.make_move(mv);
        let score = -qsearch::<Node::Next>(pos, ply + 1, -beta, -alpha, thread);
        pos.unmake_move();
        moves_seen += 1;

        if thread.abort_now {
            return Score::ZERO;
        }

        if Node::PV && (moves_seen == 1 || score > alpha) {
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
    }

    max.max(alpha)
}
