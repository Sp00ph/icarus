use crate::engine::Engine;

pub mod engine;
pub mod pesto;
pub mod position;
pub mod score;
pub mod search;
pub mod uci;
pub mod util;

fn main() -> Result<(), rootcause::Report> {
    Engine::new().run()
}
