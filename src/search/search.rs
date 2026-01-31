use arrayvec::ArrayVec;
use icarus_board::{board::TerminalState, r#move::Move};
use smallvec::SmallVec;

use crate::{
    pesto::eval,
    position::Position,
    score::Score,
    search::{
        lmr::get_lmr,
        move_picker::{MovePicker, Stage},
        searcher::ThreadCtx,
        transposition_table::TTFlag,
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

fn update_pv(thread: &mut ThreadCtx, ply: u16, mv: Move) {
    let [parent, child] = thread
        .search_stack
        .get_disjoint_mut([ply as usize, ply as usize + 1])
        .unwrap();

    parent.pv = ArrayVec::from_iter([mv]);
    parent.pv.extend(child.pv.iter().copied());
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
    if !Node::ROOT && Node::PV {
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
    let singular = thread.search_stack[ply as usize].singular;
    let singular_search = singular.is_some();

    // TT cutoffs
    if !Node::PV
        && !singular_search
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

    thread.search_stack[ply as usize].static_eval = static_eval;
    let improving = if in_check {
        false
    } else if ply >= 2 && thread.search_stack[ply as usize - 2].static_eval != -Score::INFINITE {
        static_eval > thread.search_stack[ply as usize - 2].static_eval
    } else if ply >= 4 && thread.search_stack[ply as usize - 4].static_eval != -Score::INFINITE {
        static_eval > thread.search_stack[ply as usize - 4].static_eval
    } else {
        true
    };

    if !Node::PV && !in_check && !singular_search {
        // RFP
        let rfp_depth = 6;
        let rfp_margin = 80;
        if depth < rfp_depth && static_eval - rfp_margin * (depth - improving as i16).max(0) >= beta
        {
            return static_eval;
        }

        // NMP
        let nmp_depth = 3;
        if depth >= nmp_depth && static_eval >= beta && pos.prev_move(1).is_some() {
            pos.make_null_move();
            let nmp_reduction = nmp_depth + depth / 3;
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

    // For quiet + tactic hist
    let mut quiets = SmallVec::<[Move; 64]>::new();
    let mut tactics = SmallVec::<[Move; 64]>::new();

    while let Some(mv) = move_picker.next(pos, thread) {
        if singular.is_some_and(|s| mv == s) {
            continue;
        }

        let is_tactic = pos.board().is_tactic(mv);
        let lmr = get_lmr(is_tactic, depth as u8, moves_seen);
        let mut extension = 0;
        let mut score;

        if !Node::ROOT && !best_score.is_loss() {
            if is_tactic {
                // Tactic SEE Pruning
                let tactic_base = 0;
                let tactic_scale = -60;
                let see_margin = tactic_base + tactic_scale * depth;
                if !Node::PV
                    && depth <= 10
                    && move_picker.stage() > Stage::YieldGoodNoisy
                    && !pos.cmp_see(mv, see_margin)
                {
                    continue;
                }
            } else {
                let lmr_depth = (depth - lmr).max(0);

                if !move_picker.no_more_quiets() {
                    // LMP
                    let lmp_margin =
                        (4096 + 1024 * (lmr_depth as u32).pow(2)) >> u32::from(!improving);

                    if moves_seen as u32 * 1024 >= lmp_margin {
                        move_picker.skip_quiets();
                    }

                    // FP
                    let fp_depth = 8;
                    let fp_base = 100;
                    let fp_scale = 80;

                    let fp_margin = fp_base + fp_scale * lmr_depth;
                    if !Node::PV
                        && lmr_depth <= fp_depth
                        && !in_check
                        && static_eval + fp_margin <= alpha
                    {
                        move_picker.skip_quiets();
                    }

                    // History pruning
                    let hist = thread.history.score_quiet(pos, mv);
                    let hist_scale = 2000;
                    let hist_margin = -hist_scale * lmr_depth;
                    if depth <= 5 && hist < hist_margin {
                        move_picker.skip_quiets();
                    }
                }

                // Quiet SEE Pruning
                let quiet_base = 0;
                let quiet_scale = -100;
                let see_margin = quiet_base + quiet_scale * lmr_depth;
                if !Node::PV && lmr_depth <= 10 && !pos.cmp_see(mv, see_margin) {
                    continue;
                }
            }
        }

        if !Node::ROOT
            && !singular_search
            && depth >= 8
            && let Some(tte) = tt_entry
            && tte.mv.is_some_and(|tt_mv| tt_mv == mv)
            && tte.depth as i16 >= depth - 3
            && tte.flags.tt_flag() != TTFlag::Upper
        {
            let s_beta = (tte.score - depth * 32 / 16).max(-Score::MAX_MATE + 1);
            let s_depth = (depth - 1) / 2;

            thread.search_stack[ply as usize].singular = Some(mv);
            let score = search::<NonPV>(pos, s_depth, ply, s_beta - 1, s_beta, thread);
            thread.search_stack[ply as usize].singular = None;

            if score < s_beta {
                extension = 1;
                // double extension
                let dext_margin = 20;
                extension += i16::from(!Node::PV && score + dext_margin < beta);
            } else if s_beta >= beta {
                return s_beta;
            } else if tte.score >= beta {
                // negext
                extension = -1;
            }
        }

        let initial_nodes = thread.nodes.local();
        let new_depth = depth + extension - 1;
        pos.make_move(mv);

        // PVS
        if moves_seen == 0 {
            score = -search::<Node::Next>(pos, new_depth, ply + 1, -beta, -alpha, thread);
        } else {
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

        if Node::ROOT {
            thread.root_move_nodes[mv.from()][mv.to()] += thread.nodes.local() - initial_nodes;
        }

        if thread.abort_now {
            return Score::ZERO;
        }

        if Node::ROOT && moves_seen == 1 {
            update_pv(thread, ply, mv);
        }

        if score > best_score {
            best_score = score;
        }

        if score > alpha {
            alpha = score;
            best_move = Some(mv);
            flag = TTFlag::Exact;

            if Node::PV {
                update_pv(thread, ply, mv);
            }
        }

        if score >= beta {
            flag = TTFlag::Lower;
            thread.history.update(pos, mv, &quiets, &tactics, depth);
            break;
        }

        if best_move != Some(mv) {
            if is_tactic {
                tactics.push(mv);
            } else {
                quiets.push(mv);
            }
        }
    }

    if !singular_search {
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
    }

    if !in_check
        && !singular_search
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
    if !Node::ROOT && Node::PV {
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
    let tt_entry = thread.global.ttable.fetch(pos.board().hash(), ply);

    if !in_check {
        let raw_eval = tt_entry
            .map(|e| e.eval)
            .unwrap_or_else(|| eval(pos.board()));
        let static_eval = raw_eval + thread.history.corr(pos.board());

        if static_eval >= beta {
            return static_eval;
        }

        if static_eval >= alpha {
            alpha = static_eval;
        }
    }

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

    let mut best_score = -Score::INFINITE;
    let mut moves_seen = 0;
    let mut move_picker = MovePicker::new(None, !in_check, 0, true);

    while let Some(mv) = move_picker.next(pos, thread) {
        if !best_score.is_loss() && !in_check && moves_seen > 2 {
            break;
        }

        pos.make_move(mv);
        let score = -qsearch::<Node::Next>(pos, ply + 1, -beta, -alpha, thread);
        pos.unmake_move();
        moves_seen += 1;

        if thread.abort_now {
            return Score::ZERO;
        }

        if Node::ROOT && moves_seen == 1 {
            update_pv(thread, ply, mv);
        }

        if score > best_score {
            best_score = score;
        }

        if score > alpha {
            alpha = score;

            if Node::PV {
                update_pv(thread, ply, mv);
            }
        }

        if score >= beta {
            break;
        }
    }

    best_score.max(alpha)
}
