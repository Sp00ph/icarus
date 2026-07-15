set unstable
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

host_target := `rustc --print host-tuple`
ext := if os() == "windows" { ".exe" } else { "" }
HOME := x"~"

download-net:
    python download-net.py

[default]
build-native: (build "native")

build-raw $RUSTFLAGS: download-net
    cargo build --release --target={{ host_target }}

build arch outname="icarus" $ICARUS_RELEASE="":
    just build-raw "--remap-path-prefix={{HOME}}=/ -Ctarget-cpu={{ arch }}"
    cp target/{{ host_target }}/release/icarus{{ ext }} {{ outname }}{{ ext }}

bench: build-native
    ./icarus{{ ext }} bench

run: build-native
    ./icarus{{ ext }}

build-release arch suffix:
    just build {{ arch }} icarus-{{ os() }}-{{ suffix }} 1

build-x86-releases: (build-release "x86-64" "generic") (build-release "x86-64-v3" "avx2") (build-release "znver5" "avx512")
