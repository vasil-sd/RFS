# tool_resolution (Phase 17 task 8.4): a Makefile with an explicit gcc/clang
# choice. It generates the C FSM from pass.rfs and compiles it warning-free
# under whichever compiler $CC names - `make` uses gcc (the default),
# `make CC=clang` builds the SAME generated source under clang. This mirrors
# the generated-C compiler-agnosticism guarantee: the codegen output carries no
# compiler-specific constructs, so both compilers accept it with -Wall clean.
#
# Toolchain resolution mirrors rfs-build: RFS_BIN env var > bare `rfs` on PATH;
# override with `make RFS=/path/to/rfs`. A cold checkout must run `cargo build`
# first (produces target/debug/rfs) or set RFS_BIN. The C compiler honors the
# standard CC variable; override with `make CC=/path/to/clang`.

RFS ?= $(if $(RFS_BIN),$(RFS_BIN),rfs)
CC ?= gcc
SPEC := pass.rfs
BUILD_DIR := build
GEN_FSM := $(BUILD_DIR)/fsm.c
GEN_FSM_H := $(BUILD_DIR)/fsm.h
OBJ := $(BUILD_DIR)/fsm.o

.PHONY: all generate clean

all: $(OBJ)
	@echo "built $(OBJ) with CC=$(CC)"

# Generation only (no compiler needed): the C FSM header/source pair.
generate: $(GEN_FSM) $(GEN_FSM_H)

$(GEN_FSM) $(GEN_FSM_H): $(SPEC)
	mkdir -p $(BUILD_DIR)
	$(RFS) codegen c production $(SPEC) --out-fsm-header $(GEN_FSM_H) --out-fsm-code $(GEN_FSM)

# Compile the generated FSM warning-free under $CC (gcc by default, clang via
# `make CC=clang`). The zero-warning bar is the point: the same source must be
# clean under both compilers. -I. lets the generated source's #include
# "build/fsm.h" resolve from the example directory (the path codegen stamped).
$(OBJ): $(GEN_FSM) $(GEN_FSM_H)
	$(CC) -Wall -I. -c $(GEN_FSM) -o $@

clean:
	rm -rf $(BUILD_DIR)
