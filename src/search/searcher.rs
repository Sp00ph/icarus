use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicU64, Ordering::Relaxed},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use arrayvec::ArrayVec;
use icarus_board::r#move::Move;

use crate::{
    position::Position,
    score::Score,
    search::{search::search, time_manager::TimeManager},
    uci::SearchLimit,
    util::{
        MAX_PLY,
        buffered_counter::BufferedCounter,
        command_channel::{Receiver, Sender, channel},
    },
};

// While we don't support SMP yet, to make implementing it easier, we just split global and local data already.

pub struct GlobalCtx {
    pub time_manager: TimeManager,
    /// Estimate of number of nodes searched across all threads.
    pub nodes: Arc<AtomicU64>,
    /// Number of currently searching threads + 1.
    /// If not in search, 0.
    pub num_searching: AtomicU32,
}

pub type PrincipalVariation = ArrayVec<Move, { MAX_PLY as usize }>;

pub struct ThreadCtx {
    pub id: usize,
    pub global: Arc<GlobalCtx>,
    pub chess960: bool,
    pub abort_now: bool,

    pub nodes: BufferedCounter,
    pub root_moves: Vec<Move>,
    pub sel_depth: u16,
    pub search_stack: Box<[SearchStackEntry; MAX_PLY as usize + 1]>,
    pub root_pv: PrincipalVariation,
}

#[derive(Clone, Default, Debug)]
pub struct SearchStackEntry {
    pub pv: PrincipalVariation,
}

#[derive(Clone)]
struct SearchParams {
    pos: Position,
    root_moves: Option<Vec<Move>>,
    chess960: bool,
    print_info: bool,
}

#[derive(Clone)]
enum ThreadCmd {
    Search(Box<SearchParams>),
    NewGame,
    Quit,
}

pub struct Searcher {
    pub global_ctx: Arc<GlobalCtx>,
    // TODO: Make multithreaded
    search_thread: Option<JoinHandle<()>>,
    command_sender: Sender<ThreadCmd>,
}

impl Default for Searcher {
    fn default() -> Self {
        let global_ctx = Arc::new(GlobalCtx {
            time_manager: TimeManager::default(),
            nodes: Arc::new(AtomicU64::new(0)),
            num_searching: AtomicU32::new(0),
        });
        let (tx, rx) = channel(1);
        let search_thread = Some(thread::spawn({
            let global_ctx = global_ctx.clone();
            move || {
                if std::panic::catch_unwind(move || worker_thread_loop(rx, global_ctx, 0)).is_err()
                {
                    std::process::exit(-1);
                }
            }
        }));

        Self {
            global_ctx,
            search_thread,
            command_sender: tx,
        }
    }
}

impl Searcher {
    pub fn is_running(&self) -> bool {
        self.global_ctx.num_searching.load(Relaxed) != 0
    }

    pub fn search(
        &mut self,
        pos: Position,
        limits: Vec<SearchLimit>,
        chess960: bool,
        print_info: bool,
    ) {
        assert!(
            !self.is_running(),
            "Called `search()` while already searching"
        );

        self.global_ctx.nodes.store(0, Relaxed);
        // We store one "pseudo"-searcher, to make sure that `is_running` never falsely
        // returns false
        self.global_ctx.num_searching.store(1, Relaxed);
        self.global_ctx
            .time_manager
            .init(pos.board().stm(), &limits);

        let root_moves = limits.into_iter().find_map(|limit| match limit {
            SearchLimit::SearchMoves(moves) => Some(moves),
            _ => None,
        });

        let params = Box::new(SearchParams {
            pos,
            root_moves,
            chess960,
            print_info,
        });

        self.command_sender.send(ThreadCmd::Search(params));
    }

    pub fn newgame(&mut self) {
        assert!(!self.is_running(), "Called `newgame()` while searching");
        self.command_sender.send(ThreadCmd::NewGame);
    }

    pub fn quit(&mut self) {
        self.global_ctx.time_manager.set_stop_flag(true);
        self.command_sender.send(ThreadCmd::Quit);
        self.search_thread.take().unwrap().join().unwrap();
    }

    pub fn stop(&self) {
        assert!(self.is_running());
        self.global_ctx.time_manager.set_stop_flag(true);
    }

    /// Suspends the calling thread until this search is over
    pub fn wait(&self) {
        let mut num_searching = self.global_ctx.num_searching.load(Relaxed);
        while num_searching != 0 {
            atomic_wait::wait(&self.global_ctx.num_searching, num_searching);
            num_searching = self.global_ctx.num_searching.load(Relaxed);
        }
    }
}

fn worker_thread_loop(mut rx: Receiver<ThreadCmd>, global: Arc<GlobalCtx>, id: usize) {
    let nodes = global.nodes.clone();
    let mut thread_ctx = ThreadCtx {
        global,
        id,
        chess960: false,
        nodes: BufferedCounter::new(nodes),
        root_moves: vec![],
        sel_depth: 0,
        abort_now: false,
        search_stack: vec![Default::default(); MAX_PLY as usize + 1]
            .try_into()
            .unwrap(),
        root_pv: Default::default(),
    };

    loop {
        match rx.recv(|cmd| cmd.clone()) {
            ThreadCmd::Search(search_params) => {
                thread_ctx.global.num_searching.fetch_add(1, Relaxed);
                thread_ctx.nodes.reset_local();
                thread_ctx.root_moves = search_params
                    .root_moves
                    .unwrap_or_else(|| search_params.pos.board().gen_all_moves_to());
                thread_ctx.chess960 = search_params.chess960;
                thread_ctx.search_stack.fill(Default::default());
                thread_ctx.abort_now = false;

                id_loop(search_params.pos, &mut thread_ctx, search_params.print_info);
            }
            // We don't have anything to clear on newgame yet.
            ThreadCmd::NewGame => {}
            ThreadCmd::Quit => return,
        }
    }
}

fn id_loop(mut pos: Position, thread: &mut ThreadCtx, print: bool) {
    let mut depth = 1;
    let mut overall_best_score = -Score::INFINITE;

    loop {
        thread.sel_depth = 0;
        let new_score = search::<true>(
            &mut pos,
            depth as i32,
            0,
            -Score::INFINITE,
            Score::INFINITE,
            thread,
        );
        thread.nodes.flush();

        if depth > 1 && thread.abort_now {
            break;
        }

        thread.root_pv = thread.search_stack[0].pv.clone();

        if print && thread.id == 0 {
            print_info(new_score, depth, thread);
        }
        overall_best_score = new_score;

        if depth >= MAX_PLY
            || thread
                .global
                .time_manager
                .stop_id(depth, thread.nodes.global())
        {
            break;
        }

        depth += 1;
    }

    if thread.global.time_manager.infinite() {
        // Lazy way of waiting for the stop flag to be set. Ideally, we'd wait here with a futex or condvar.
        while !thread.global.time_manager.stop_flag() {
            thread::sleep(Duration::from_millis(10));
        }
    }

    // If we are the last thread to decrement, then set num_searching to 0
    // to signal that no thread is still searching.
    let last = thread.global.num_searching.fetch_sub(1, Relaxed) == 2;
    if last {
        thread.global.num_searching.store(0, Relaxed);
    }

    let best_move = *thread
        .root_pv
        .first()
        .or(thread.root_moves.first())
        .unwrap();

    if print && thread.id == 0 {
        print_info(overall_best_score, depth, thread);
        println!("bestmove {}", best_move.display(thread.chess960));
    }

    // We want the waiters to wake up after the bestmove print
    if last {
        atomic_wait::wake_all(&thread.global.num_searching);
    }
}

fn print_info(score: Score, depth: u16, thread: &ThreadCtx) {
    let nodes = thread.nodes.global();
    let time_us = thread.global.time_manager.elapsed().as_micros();
    let nps = ((nodes as f64) / (time_us.max(1) as f64) * 1e6) as u64;
    let time_ms = time_us / 1000;
    let pv = {
        use std::fmt::Write;
        let mut s = String::new();
        for mv in &thread.root_pv {
            write!(s, "{} ", mv.display(thread.chess960)).unwrap();
        }
        s.pop();
        s
    };
    println!(
        "info depth {} seldepth {} score {} time {} nodes {} nps {} pv {}",
        depth, thread.sel_depth, score, time_ms, nodes, nps, pv
    )
}
