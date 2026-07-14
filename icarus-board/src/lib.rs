pub mod attack_generators;
pub mod board;
pub mod castling;
pub mod ep_file;
pub mod is_legal;
pub mod r#move;
pub mod movegen;
pub mod perft;
pub mod zobrist;

cfg_select! {
    target_feature = "avx512f" => {
        #[path = "setwise_attacks/avx512.rs"]
        pub mod setwise_attacks;
    }
    target_feature = "avx2" => {
        #[path = "setwise_attacks/avx2.rs"]
        pub mod setwise_attacks;
    }
    _ => {
        #[path = "setwise_attacks/generic.rs"]
        pub mod setwise_attacks;
    }
}
