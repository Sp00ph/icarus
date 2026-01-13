use crate::engine::Engine;

pub mod bench;
pub mod engine;
pub mod pesto;
pub mod position;
pub mod score;
pub mod search;
pub mod uci;
pub mod util;

#[cfg(feature = "test-islegal")]
pub mod test_islegal;

fn main() -> Result<(), rootcause::Report> {
    Engine::new().run()
}
