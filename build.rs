use std::{env, fs, path::PathBuf, str::FromStr};

use curl::easy::Easy;
use sha2::{Digest, Sha256};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=network.txt");
    println!("cargo:rerun-if-env-changed=EVALFILE");

    let output_path = PathBuf::from_str(&env::var("OUT_DIR").unwrap())
        .unwrap()
        .join("icarus.nnue");

    if let Ok(path) = env::var("EVALFILE")
        && !path.is_empty()
    {
        fs::copy(path, output_path).unwrap();
    } else {
        let contents = fs::read_to_string("network.txt").unwrap();
        let (name, hash) = contents.trim().split_once(' ').unwrap();
        let hash = hex::decode(hash).unwrap();
        if let Ok(bytes) = fs::read(&output_path)
            && Sha256::digest(&bytes)[..] == hash[..]
        {
            return;
        }

        let mut network = vec![];
        let mut curl = Easy::new();
        curl.follow_location(true).unwrap();
        curl.url(&format!(
            "https://github.com/Sp00ph/icarus-nets/releases/download/{name}/{name}.nnue"
        ))
        .unwrap();
        curl.get(true).unwrap();
        {
            let mut transfer = curl.transfer();
            transfer
                .write_function(|data| {
                    network.extend_from_slice(data);
                    Ok(data.len())
                })
                .unwrap();
            transfer.perform().unwrap();
        }

        assert!(
            Sha256::digest(&network)[..] == hash[..],
            "Incorrect network hash in network.txt"
        );

        fs::write(output_path, network).unwrap();
    }
}
