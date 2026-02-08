ifeq ($(EXE),)
EXE = icarus
endif

EXT = 
ifeq ($(OS),Windows_NT)
EXT := .exe
endif
.PHONY: native

native:
ifndef EVALFILE
	python3 ./download-net.py
endif
	cargo build --release -p icarus
	cp target/release/icarus$(EXT) ./$(EXE)$(EXT)
