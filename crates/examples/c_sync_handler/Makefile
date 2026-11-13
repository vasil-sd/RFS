# Plain-C sync-style ARQ handler on the generated spec-43 FSM (Phase 5 task
# 12) - the C mirror of crates/examples/sync_handler: same scenario, same
# pinned outcome. Makefile-driven by design (NOT a cargo workspace member).
#
# Toolchain resolution mirrors rfs-build: RFS_BIN env var > bare `rfs` on
# PATH; override with `make RFS=/path/to/rfs`. A cold checkout must run
# `cargo build` first (produces target/debug/rfs) or set RFS_BIN. The C
# compiler honors the standard CC variable (Phase 12 portability - NixOS /
# nix-shell export CC); override with `make CC=/path/to/gcc`.

RFS ?= $(if $(RFS_BIN),$(RFS_BIN),rfs)
CC ?= gcc
SPEC := ../../../examples/valid/43_sliding_window.rfs
BUILD_DIR := build
GEN_FSM := $(BUILD_DIR)/arq_fsm.c
GEN_FSM_H := $(BUILD_DIR)/arq_fsm.h

.PHONY: all run generate clean

all: c_sync_handler

# Generation only (no gcc needed): the C FSM header/source pair under the name
# handler.c includes. Phase 14 task 2: production is a split - the source
# #includes its own header, so BOTH files must be generated.
generate: $(GEN_FSM) $(GEN_FSM_H)

$(GEN_FSM) $(GEN_FSM_H): $(SPEC)
	mkdir -p $(BUILD_DIR)
	$(RFS) codegen c production $(SPEC) --out-fsm-header $(GEN_FSM_H) --out-fsm-code $(GEN_FSM)

c_sync_handler: src/handler.c $(GEN_FSM) $(GEN_FSM_H)
	$(CC) -Wall -I$(BUILD_DIR) -I. -o $@ src/handler.c

run: c_sync_handler
	./c_sync_handler

clean:
	rm -rf $(BUILD_DIR) c_sync_handler
