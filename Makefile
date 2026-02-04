EXE = icarus
EXT = 
ifeq ($(OS),Windows_NT)
EXT := .exe
endif

native: export HOST_CC = clang
native:	
	cargo +stable build --release -p icarus
	cp target/release/icarus$(EXT) ./$(EXE)$(EXT)