use arrayvec::ArrayVec;
use icarus_board::{board::TerminalState, r#move::Move};
use smallvec::SmallVec;

use crate::{
    position::Position,
    score::Score,
    search::{
        move_picker::{MovePicker, Stage},
        params::*,
        searcher::ThreadCtx,
        transposition_table::TTFlag,
    },
    util::MAX_PLY,
};

pub const DEPTH_SCALE: i32 = 1024;

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
    mut depth: i32,
    ply: u16,
    mut alpha: Score,
    mut beta: Score,
    cutnode: bool,
    thread: &mut ThreadCtx,
) -> Score {
    if !Node::ROOT && (thread.abort_now || thread.global.time_manager.stop_search(thread)) {
        if thread.id == 0 {
            thread.global.time_manager.set_stop_flag(true);
        }
        thread.abort_now = true;
        return Score::ZERO;
    }

    thread.sel_depth = thread.sel_depth.max(ply);
    if Node::PV {
        thread.search_stack[ply as usize].pv.clear();
    }

    // Mate distance pruning
    if !Node::ROOT {
        alpha = alpha.max(Score::new_mated(ply));
        beta = beta.min(Score::new_mate(ply + 1));

        if alpha >= beta {
            thread.nodes.inc();
            return alpha;
        }
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
    let tt_pv = Node::PV || tt_entry.is_some_and(|e| e.flags.pv());
    let singular = thread.search_stack[ply as usize].singular;
    let singular_search = singular.is_some();

    // TT cutoffs
    if !Node::PV
        && !singular_search
        && let Some(e) = tt_entry
        && e.depth as i32 * DEPTH_SCALE >= depth
    {
        let score = e.score;
        match e.flags.tt_flag() {
            TTFlag::Exact => return score,
            TTFlag::Lower if score >= beta => return score,
            TTFlag::Upper if score <= alpha => return score,
            _ => {}
        }
    }

    let in_check = pos.board().checkers().is_non_empty();

    let (raw_eval, static_eval) = if in_check {
        (Score::NONE, Score::NONE)
    } else if singular_search {
        (Score::NONE, thread.search_stack[ply as usize].static_eval)
    } else {
        let raw_eval = tt_entry
            .map(|e| e.eval)
            .unwrap_or_else(|| pos.eval(&mut thread.nnue));
        let static_eval = Score::clamp_nomate(raw_eval.0.saturating_add(thread.history.corr(pos)));
        (raw_eval, static_eval)
    };

    thread.search_stack[ply as usize].static_eval = static_eval;
    thread.search_stack[ply as usize + 2].cutoffs = 0;
    let improving = if in_check {
        false
    } else if ply >= 2 && thread.search_stack[ply as usize - 2].static_eval != Score::NONE {
        static_eval > thread.search_stack[ply as usize - 2].static_eval
    } else if ply >= 4 && thread.search_stack[ply as usize - 4].static_eval != Score::NONE {
        static_eval > thread.search_stack[ply as usize - 4].static_eval
    } else {
        true
    };

    // Hindsight ext
    if !Node::ROOT
        && !in_check
        && !singular_search
        && thread.search_stack[ply as usize - 1].reduction >= hindsight_ext_min_red()
        && thread.search_stack[ply as usize - 1].static_eval != Score::NONE
        && static_eval < -thread.search_stack[ply as usize - 1].static_eval
    {
        depth += hindsight_ext_ext();
    }

    if !Node::PV && !in_check && !singular_search {
        // RFP
        let improving_depth = (depth / DEPTH_SCALE - improving as i32).max(0) as i16;
        if depth < rfp_depth()
            && !beta.is_win()
            && static_eval
                - rfp_margin() * improving_depth
                - rfp_quad_margin() * improving_depth.pow(2) / 128
                >= beta
        {
            return Score(static_eval.0.midpoint(beta.0));
        }

        // NMP
        if depth >= nmp_depth()
            && ply >= thread.min_nmp_ply
            && static_eval >= beta
            && pos.prev_move(1).is_some()
            && cutnode
        {
            pos.make_null_move();
            thread.global.ttable.prefetch(pos.board());

            let nmp_reduction = nmp_red_base() + depth * 128 / nmp_red_scale_div();
            let score = -search::<NonPV>(
                pos,
                depth - nmp_reduction,
                ply + 1,
                -beta,
                -beta + 1,
                !cutnode,
                thread,
            );
            pos.unmake_null_move();

            if thread.abort_now {
                return Score::ZERO;
            }

            if score >= beta {
                if depth <= nmp_verif_min_depth() || thread.min_nmp_ply > 0 {
                    if score.is_win() {
                        return beta;
                    } else {
                        return score;
                    }
                }

                thread.min_nmp_ply =
                    ply + ((depth - nmp_reduction).max(0) / DEPTH_SCALE) as u16 * 3 / 4;
                let verif_score = search::<NonPV>(
                    pos,
                    depth - nmp_reduction,
                    ply,
                    beta - 1,
                    beta,
                    true,
                    thread,
                );
                thread.min_nmp_ply = 0;

                if verif_score >= beta {
                    return verif_score;
                }
            }
        }
    }

    let probcut_beta = beta.saturating_add(probcut_margin());
    if !Node::PV
        && !singular_search
        && !in_check
        && let Some(tte) = tt_entry
        && tte.score != Score::NONE
        && !tte.score.is_mate()
        && !beta.is_mate()
        && matches!(tte.flags.tt_flag(), TTFlag::Lower | TTFlag::Exact)
        && tte.score >= probcut_beta
        && (tte.depth as i32) * DEPTH_SCALE >= depth - probcut_depth_offset()
    {
        return tte.score;
    }

    let mut move_picker = MovePicker::new(tt_move, false, movepick_see_threshold());
    let mut best_score = -Score::INFINITE;
    let mut moves_seen = 0;
    let mut best_move = None;
    let mut flag = TTFlag::Upper;

    // For quiet + tactic hist
    let mut quiets = SmallVec::<[Move; 64]>::new();
    let mut tactics = SmallVec::<[Move; 64]>::new();

    while let Some(mv) = move_picker.next(pos, thread) {
        if singular == Some(mv) {
            continue;
        }

        let is_tactic = pos.board().is_tactic(mv);
        let mut lmr = get_lmr(is_tactic, (depth / DEPTH_SCALE) as u8, moves_seen);
        let mut extension = 0;
        let mut score;

        if !Node::ROOT && !best_score.is_loss() {
            if is_tactic {
                // Tactic SEE Pruning
                let see_margin =
                    tactic_see_base() + (tactic_see_scale() * depth / DEPTH_SCALE) as i16;
                if !Node::PV
                    && depth <= see_max_depth()
                    && move_picker.stage() > Stage::YieldGoodNoisy
                    && !pos.cmp_see(mv, see_margin)
                {
                    continue;
                }
            } else {
                let lmr_depth = (depth - lmr).max(0);

                if !move_picker.no_more_quiets() {
                    // LMP
                    let lmp_margin = (lmp_base()
                        + lmp_scale() * ((lmr_depth / DEPTH_SCALE) as u32).pow(2))
                        >> u32::from(!improving);

                    if moves_seen as u32 * 1024 >= lmp_margin {
                        move_picker.skip_quiets();
                    }

                    // FP
                    let fp_margin = fp_base() + (fp_scale() * lmr_depth / DEPTH_SCALE) as i16;
                    if !Node::PV
                        && lmr_depth <= fp_depth()
                        && !in_check
                        && static_eval + fp_margin <= alpha
                    {
                        move_picker.skip_quiets();
                    }

                    // History pruning
                    let hist = thread.history.score_quiet(pos, mv);
                    let hist_margin = -hist_prune_scale() * lmr_depth / DEPTH_SCALE;
                    if depth <= hist_prune_depth() && (hist as i32) < hist_margin {
                        move_picker.skip_quiets();
                        continue;
                    }
                }

                // Quiet SEE Pruning
                let see_margin =
                    quiet_see_base() + (quiet_see_scale() * lmr_depth / DEPTH_SCALE) as i16;
                if !Node::PV && lmr_depth <= see_max_depth() && !pos.cmp_see(mv, see_margin) {
                    continue;
                }
            }
        }

        if !Node::ROOT
            && !singular_search
            && depth >= se_min_depth()
            && let Some(tte) = tt_entry
            && tte.mv.is_some_and(|tt_mv| tt_mv == mv)
            && tte.depth as i32 * DEPTH_SCALE >= (depth - se_tt_depth_offset())
            && tte.flags.tt_flag() != TTFlag::Upper
        {
            let s_beta = tte
                .score
                .saturating_add((-depth * se_beta_scale() / (DEPTH_SCALE * 128)) as i16)
                .max(-Score::MAX_MATE + 1);
            let s_depth = (depth - se_depth_offset()) * se_depth_scale() / 128;

            thread.search_stack[ply as usize].singular = Some(mv);
            let score = search::<NonPV>(pos, s_depth, ply, s_beta - 1, s_beta, cutnode, thread);
            thread.search_stack[ply as usize].singular = None;

            if score < s_beta {
                extension = se_single_ext();
                // double extension
                extension +=
                    se_double_ext() * i32::from(!Node::PV && score + se_dext_margin() < beta);
            } else if s_beta >= beta {
                return s_beta;
            } else if tte.score >= beta {
                extension = se_triple_negext();
            } else if cutnode {
                // double negext
                extension = se_double_negext();
            } else if tte.score <= alpha {
                // negext
                extension = se_single_negext();
            }
        }

        let initial_nodes = thread.nodes.local();
        let new_depth = depth + extension - DEPTH_SCALE;

        let hist_lmr = if pos.board().is_quiet(mv) {
            thread.history.score_quiet(pos, mv) / quiet_hist_lmr_div()
        } else {
            0
        };

        pos.make_move(mv, Some(&mut thread.nnue));
        thread.global.ttable.prefetch(pos.board());

        // PVS
        if moves_seen == 0 {
            score = -search::<Node::Next>(
                pos,
                new_depth,
                ply + 1,
                -beta,
                -alpha,
                !Node::PV && !cutnode,
                thread,
            );
        } else {
            if depth < lmr_min_depth() {
                lmr = 0;
            } else {
                lmr += lmr_nonpv() * !Node::PV as i32;
                lmr -= lmr_ttpv() * tt_pv as i32;
                lmr -= lmr_check() * pos.board().checkers().is_non_empty() as i32;
                lmr += lmr_cutnode() * cutnode as i32;
                lmr += lmr_cutoffs() * (thread.search_stack[ply as usize + 1].cutoffs > 3) as i32;
                lmr -= DEPTH_SCALE * hist_lmr as i32;
            }

            let lmr_depth = (new_depth - lmr).max(DEPTH_SCALE).min(new_depth);

            thread.search_stack[ply as usize].reduction = lmr;
            score = -search::<NonPV>(pos, lmr_depth, ply + 1, -alpha - 1, -alpha, true, thread);
            thread.search_stack[ply as usize].reduction = 0;

            if lmr > 0 && score > alpha {
                score = -search::<NonPV>(
                    pos,
                    new_depth,
                    ply + 1,
                    -alpha - 1,
                    -alpha,
                    !cutnode,
                    thread,
                )
            }
            if Node::PV && score > alpha {
                score = -search::<PV>(pos, new_depth, ply + 1, -beta, -alpha, false, thread);
            }
        }

        pos.unmake_move(Some(&mut thread.nnue));
        moves_seen += 1;

        if Node::ROOT {
            thread.root_move_nodes[mv.from()][mv.to()] += thread.nodes.local() - initial_nodes;

            if moves_seen == 1 {
                update_pv(thread, ply, mv);
            }
        }

        if thread.abort_now {
            return Score::ZERO;
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
            thread
                .history
                .update(pos, mv, &quiets, &tactics, (depth / DEPTH_SCALE) as i16);
            break;
        }

        if best_move != Some(mv) {
            if is_tactic {
                tactics.push(mv);
            } else {
                quiets.push(mv);
            }
        }
        thread.search_stack[ply as usize].cutoffs += 1;
    }

    if !singular_search {
        thread.global.ttable.store(
            pos.board().hash(),
            (depth / DEPTH_SCALE) as u8,
            ply,
            raw_eval,
            best_score,
            best_move,
            flag,
            tt_pv,
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
            .update_corr(pos, (depth / DEPTH_SCALE) as i16, best_score, static_eval);
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
    if thread.abort_now || thread.global.time_manager.stop_search(thread) {
        thread.abort_now = true;
        return Score::ZERO;
    }
    thread.nodes.inc();
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
        return pos.eval(&mut thread.nnue);
    }

    let in_check = pos.board().checkers().is_non_empty();
    let tt_entry = thread.global.ttable.fetch(pos.board().hash(), ply);

    let mut static_eval = Score::new_mated(ply);

    if !in_check {
        let raw_eval = tt_entry
            .map(|e| e.eval)
            .unwrap_or_else(|| pos.eval(&mut thread.nnue));
        static_eval = raw_eval + thread.history.corr(pos);

        if static_eval >= beta {
            if static_eval.max(beta).is_win() {
                return static_eval;
            } else {
                return Score(static_eval.0.midpoint(beta.0));
            }
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

    let mut best_score = static_eval;
    let mut moves_seen = 0;
    let mut move_picker = MovePicker::new(None, !in_check, qs_see_threshold());

    while let Some(mv) = move_picker.next(pos, thread) {
        if !best_score.is_loss() {
            // LMP
            if !in_check && moves_seen > qs_lmp_limit() {
                break;
            }
            // SEE Pruning
            if move_picker.stage() >= Stage::YieldBadNoisy {
                break;
            }
            // Skip quiets if non-mated evasion was found
            move_picker.skip_quiets();
            if pos.board().is_quiet(mv) {
                continue;
            }
        }

        pos.make_move(mv, Some(&mut thread.nnue));
        thread.global.ttable.prefetch(pos.board());
        let score = -qsearch::<Node::Next>(pos, ply + 1, -beta, -alpha, thread);
        pos.unmake_move(Some(&mut thread.nnue));
        moves_seen += 1;

        if Node::ROOT && moves_seen == 1 {
            update_pv(thread, ply, mv);
        }

        if thread.abort_now {
            return Score::ZERO;
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
