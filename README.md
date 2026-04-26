<div align="center">

# Icarus

![LGBTQ+ friendly][lgbtqp-badge]
![trans rights][trans-rights-badge]

</div>

A superhuman UCI chess engine written in Rust, supporting standard and (Double) Fischer Random chess, with NNUE evaluation trained exclusively on self-play data. This repository contains no LLM-generated or LLM-assisted code.

## Getting started
### Precompiled Binaries
Binaries for x86-64 Windows and Linux are available on [the GitHub releases page](https://github.com/Sp00ph/icarus/releases).

- `avx512`: fastest, requires a recent AVX512-capable CPU (Ice Lake or newer on Intel, Zen 4 or newer on AMD)
- `avx2`: usable on any CPU since 2015 (Haswell or newer in Intel, Excavator or newer on AMD)
- `generic`: fallback option that will run on any x86-64 CPU. Much slower than the other builds.

> [!NOTE]
> If you're unsure which binary to use, try the AVX512 build first. If it doesn't run on your system, fall back to the AVX2 build, or the generic one as a last resort.
> 
> If you want the best possible performance, you may also build icarus from source, which will optimize for your specific CPU model.

### Building From Source
Building icarus requires [Rust](https://rustup.rs/) and [Python 3](https://www.python.org/downloads/) to be installed.

If you have GNU make installed, running `make` in the repository root will build the engine and deposit the binary in `icarus`/`icarus.exe`. To build without `make`, you should run

```bash
python download-net.py
cargo build --release --package icarus
```

The engine binary will be located in `target/release/`. On a BMI2 capable CPU, PEXT/PDEP attack generation can be enabled by passing `--feature use-bmi2` to `cargo build`. It is disabled by default, because PEXT/PDEP have horrible performance on AMD Zen and Zen 2.

### Usage
Icarus supports the UCI protocol, and is designed to be used with UCI-compatible match runners or GUIs, such as [Cute Chess](https://cutechess.com/), [fastchess](https://github.com/Disservin/fastchess/), [En Croissant](https://encroissant.org/) or [Nibbler](https://github.com/rooklift/nibbler).

### UCI Options

Icarus supports the following UCI options:

| Name           | Values     | Default | Description                                                       |
| -------------- | ---------- | ------- | ----------------------------------------------------------------- |
| `Hash`         | 1-1048576  | 16      | Transposition table size in MiB                                   |
| `Threads`      | 1-512      | 1       | Number of search threads                                          |
| `UCI_Chess960` | false,true | false   | Enable Chess960 (Fischer Random) support                          |
| `Minimal`      | false,true | false   | Show minimal UCI output                                           |
| `MoveOverhead` | 0-65535    | 20      | Time reserved for communication overhead per move in milliseconds |

In addition to the standard UCI commands, icarus also supports the following nonstandard commands:

| Command                      | Description                                                                                        |
| ---------------------------- | -------------------------------------------------------------------------------------------------- |
| `perft <depth> [false]`      | Runs a perft test to the given depth. If the second argument is `false`, it uses non-bulk counting |
| `splitperft <depth> [false]` | Same as `perft`, but reports the node counts for each move individually                            |
| `bench <depth>`              | Runs a fixed-depth search on a list of positions and reports node count and NPS                    |
| `d`                          | Displays the current position in a human-readable format                                           |
| `eval`                       | Reports the static evaluation for the current position                                             |
| `wait`                       | Blocks the UCI thread until the current search has finished                                        |

## Features
### Move Generation
- Fully legal move generation with threat bitboard computation
- PEXT/PDEP accelerated slider move generation on modern CPUs
- Black magic slider generation on older CPUs

### Search
- Negamax using Alpha-Beta Pruning
- Iterative Deepening
- Quiescence Search
- Transposition Table
- Principal Variation Search
- Reverse Futility Pruning
- Null Move Pruning
- Late Move Reduction
- Late Move Pruning
- Futility Pruning
- SEE Pruning
- Aspiration Windows
- History Pruning
- Singular Extension (thanks to @kelseyde)
    - Double Extensions
    - Negative Extensions
    - Multicut
- Multithreading using LazySMP

### Move Ordering
- Hash Move
- Quiet History
    - Threat Bucketing
- Tactic History
- Continuation History
- Staged Move Generation
- Static Exchange Evaluation

### Evaluation
- NNUE
    - Dual Perspective
    - Horizontal Mirroring
    - Trained only on self-play using [bullet](https://github.com/jw1912/bullet)
    - Initial version trained on data generated using PeSTO piece-square tables.
- Correction History
    - Pawn corrhist
    - Minor corrhist
    - Major corrhist
    - Non-pawn corrhist

## Acknowledgements
Icarus takes inspiration from other engines, including but not limited to:
- [Cherry](https://github.com/teccii/cherry)
- [Stormphrax](https://github.com/Ciekce/Stormphrax)
- [Viridithas](https://github.com/cosmobobak/viridithas)
- [Hobbes](https://github.com/kelseyde/hobbes-chess-engine)

Additionally, there are many individuals who have made developing Icarus easier and more fun, including but very much not limited to:
- [Tecci](https://github.com/teccii), author of cherry
- [Ciekce](https://github.com/Ciekce), author of Stormphrax
- [Cosmo](https://github.com/cosmobobak), author of Viridithas
- [Dan Kelsey](https://github.com/kelseyde), author of Hobbes and contributor to Icarus
- [Jonathan Hallström](https://github.com/JonathanHallstrom), author of [pawnocchio](https://github.com/JonathanHallstrom/pawnocchio) and co-author of [vine](https://github.com/vine-chess/vine)
- [lily](https://github.com/87flowers), author of Rose and SIMD wizard

Special thanks go to Jonathan Hallström for helping with generating most of the training data.


[lgbtqp-badge]: https://pride-badges.pony.workers.dev/static/v1?label=lgbtq%2B%20friendly&stripeWidth=6&stripeColors=E40303,FF8C00,FFED00,008026,24408E,732982
[trans-rights-badge]: https://pride-badges.pony.workers.dev/static/v1?label=trans%20rights&stripeWidth=6&stripeColors=5BCEFA,F5A9B8,FFFFFF,F5A9B8,5BCEFA