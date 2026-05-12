cfg_select! {
    all(target_feature = "avx512bw", target_feature = "avx512vl", target_feature = "avx512vbmi2") => {
        mod avx512;
        pub use avx512::*;
    }
    target_feature = "avx2" => {
        mod avx2;
        pub use avx2::*;
    }
    target_feature = "neon" => {
        mod neon;
        pub use neon::*;
    }
    _ => {
        compile_error!("Unsupported arch!");
    },
}
