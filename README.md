# Icarus

A superhuman UCI chess engine, supporting standard and (Double) Fischer Random chess, with NNUE evaluation trained exclusively on self-play data. This repository contains no LLM-generated or LLM-assisted code.

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