use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=icarus.nnue");
    println!("cargo:rerun-if-env-changed=EVALFILE");

    let out_path = env::var("OUT_DIR").unwrap() + "/icarus.nnue";
    let in_path = env::var("EVALFILE").unwrap_or_else(|_| "icarus.nnue".to_string());

    if !fs::exists(&in_path).unwrap() {
        panic!(
            "No net found! Use the Makefile, `download-net.py`, or specify a net path through the `EVALFILE` env var!"
        );
    }

    fs::copy(in_path, out_path).unwrap();
}
