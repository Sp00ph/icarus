use std::{env, fs, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=EVALFILE");
    let out_path = env::var("OUT_DIR").unwrap() + "/icarus.nnue";
    let in_path = env::var("EVALFILE").unwrap_or_else(|_| "nets/icarus.nnue".to_string());
    println!("cargo:rerun-if-changed={in_path}");


    if !fs::exists(&in_path).unwrap() {
        panic!(
            "No net found! Use the Makefile, `download-net.py`, or specify a net path through the `EVALFILE` env var!"
        );
    }

    fs::copy(in_path, out_path).unwrap();
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let version = version.strip_suffix(".0").unwrap_or(&version);
    let dev_suffix = if env::var("ICARUS_RELEASE").is_ok_and(|s| s == "1") {
        String::new()
    } else {
        let hash = if let Ok(out) = Command::new("git")
            .args(["rev-parse", "--short", "@"])
            .output()
            && out.status.success()
        {
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        } else {
            String::new()
        };
        format!("-dev {hash}")
    };

    println!("cargo:rustc-env=ICARUS_VERSION={version}{dev_suffix}");
}
