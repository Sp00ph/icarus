#!/bin/bash
fastchess                                                       \
    -engine cmd=engines/icarus-dev name=IcarusDev               \
    -engine cmd=engines/icarus-main name=IcarusMain             \
    -each tc=8+0.08 option.MoveOverhead=10                      \
    -rounds 50000                                               \
    -concurrency 12                                             \
    -openings order=random file=books/UHO_Lichess_4852_v1.epd   \
    -sprt elo0=0 elo1=5 alpha=0.05 beta=0.05
