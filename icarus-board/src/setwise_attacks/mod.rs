
use cfg_if::cfg_if;

cfg_if!(
    if #[cfg(target_feature = "avx512f")] {
        mod avx512;
        pub use avx512::*;
    } else if #[cfg(target_feature = "avx2")] {
        mod avx2;
        pub use avx2::*;
    } else {
        mod generic;
        pub use generic::*;
    }
);