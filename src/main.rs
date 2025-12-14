use crate::engine::Engine;

pub mod engine;
pub mod uci;

fn main() -> Result<(), rootcause::Report> {
    Engine::new().run()
}
