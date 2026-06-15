use crate::engine::Engine;

pub mod bench;
pub mod datagen;
pub mod engine;
pub mod nnue;
pub mod position;
pub mod score;
pub mod search;
pub mod uci;
pub mod util;
pub mod wdl;
pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() -> anyhow::Result<()> {
    Engine::new().run()
}
