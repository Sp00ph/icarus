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
pub mod weights;

fn main() -> Result<(), rootcause::Report> {
    Engine::new().run()
}
