# Plain-C sync-style ARQ handler on the generated spec-43 FSM (Phase 5 task
# 12) - the C mirror of crates/examples/sync_handler: same scenario, same
# pinned outcome. Makefile-driven by design (NOT a cargo workspace member).
#
# Toolchain resolution mirrors rfs-build: RFS_BIN env var > bare `rfs` on
# PATH; override with `make RFS=/path/to/rfs`. A cold checkout must run
# `cargo build` first (produces target/debug/rfs) or set RFS_BIN.

RFS ?= $(if $(RFS_BIN),$(RFS_BIN),rfs)
SPEC := ../../../examples/valid/43_sliding_window.rfs
BUILD_DIR := build
GEN_FSM := $(BUILD_DIR)/arq_fsm.c

.PHONY: all run generate clean

all: c_sync_handler

# Generation only (no gcc needed): the C FSM source under the name handler.c
# includes.
generate: $(GEN_FSM)

$(GEN_FSM): $(SPEC)
	mkdir -p $(BUILD_DIR)
	$(RFS) codegen c production $(SPEC) --out $@

c_sync_handler: src/handler.c $(GEN_FSM)
	gcc -Wall -I$(BUILD_DIR) -o $@ src/handler.c

run: c_sync_handler
	./c_sync_handler

clean:
	rm -rf $(BUILD_DIR) c_sync_handler
