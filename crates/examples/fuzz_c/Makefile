# clang + libFuzzer harness for the emitted spec-43 fuzz skeleton (Phase 5
# task 12). GENERATION needs only the rfs toolchain; BUILDING needs a
# libFuzzer-enabled clang (see README.md - generation works without it).
#
# Toolchain: RFS_BIN (or RFS) locates the rfs toolchain, mirroring rfs-build's
# resolution order (env var > PATH); CLANG overrides the C compiler.

RFS ?= $(if $(RFS_BIN),$(RFS_BIN),rfs)
CLANG ?= clang
SPEC := ../../../examples/valid/43_sliding_window.rfs
BUILD_DIR := build

.PHONY: all generate run clean

all: fuzz_target

# Generation only (no clang needed): the fsm header/source pair under the
# exact names the skeleton's #include expects, plus the libFuzzer skeleton as
# a NAMED slot of the `test` command (Phase 14 - no generate-then-delete
# dance: every artifact lands in build/ and `make clean` removes them all).
GEN_FSM_C := $(BUILD_DIR)/rfs_generated_fsm.c
GEN_FSM_H := $(BUILD_DIR)/rfs_generated_fsm.h

generate: $(BUILD_DIR)/fuzz_target.c $(GEN_FSM_C) $(GEN_FSM_H)

# Phase 14 task 2: production is a header/source pair - the source #includes
# its own header, so BOTH files must be generated.
$(GEN_FSM_C) $(GEN_FSM_H): $(SPEC)
	mkdir -p $(BUILD_DIR)
	$(RFS) codegen c production $(SPEC) --out-fsm-header $(GEN_FSM_H) --out-fsm-code $(GEN_FSM_C)

# The `test` command fills three slots: the test-executable source (kept at
# build/spec_test.c - unused by this example, cleaned with everything else),
# the fuzz skeleton (the named slot this example consumes), and the referenced
# fsm pair (take-or-emit under the form-2 names above).
$(BUILD_DIR)/fuzz_target.c: $(SPEC)
	mkdir -p $(BUILD_DIR)
	$(RFS) codegen c test $(SPEC) \
		--out-test-code $(BUILD_DIR)/spec_test.c \
		--out-fuzz-target-code $@ \
		--fsm-file-header $(GEN_FSM_H) --fsm-file-source $(GEN_FSM_C)

# All generated files live in build/; -I. lets the fsm source's self-include
# of "build/rfs_generated_fsm.h" resolve from the example dir.
# Phase 14 task 4: the skeleton references the fsm through its header, so BOTH
# the fsm source and the skeleton are compiled together (no more source-include).
fuzz_target: $(BUILD_DIR)/fuzz_target.c $(GEN_FSM_C) $(GEN_FSM_H)
	$(CLANG) -O1 -fsanitize=fuzzer,address -I. -o $@ $(GEN_FSM_C) $(BUILD_DIR)/fuzz_target.c

# Bounded smoke run (a real campaign is user work, not a build target).
run: fuzz_target
	./fuzz_target -max_total_time=10

clean:
	rm -rf $(BUILD_DIR) fuzz_target
	# Defensive: never leave a lone BASE-derived fsm file behind (a partial pair
	# is a usage error for the test command's take-or-emit reference).
	rm -f 43_sliding_window-fsm.h 43_sliding_window-fsm.c
