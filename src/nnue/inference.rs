use cfg_if::cfg_if;

use crate::nnue::network::Network;

use super::network::HL;

const QA: i16 = 255;
const QB: i16 = 64;

const SCALE: i32 = 400;

cfg_if!(
    if #[cfg(target_feature = "avx512bw")] {
        mod avx512;
        pub use avx512::forward;
    } else if #[cfg(target_feature = "avx2")] {
        mod avx2;
        pub use avx2::forward;
    } else {
        error!("Non avx2 disabled for SPRT purposes");
        mod generic;
        pub use generic::forward;
    }
);
