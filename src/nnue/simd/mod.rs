cfg_select! {
    target_feature = "avx2" => {
        mod avx2;
        pub use avx2::*;
    }
    _ => {
        compile_error!("Unsupported arch!");
    },
}
