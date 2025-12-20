use crate::engine::Engine;

pub mod engine;
pub mod uci;
pub mod position;

fn main() -> Result<(), rootcause::Report> {
    Engine::new().run()
}
