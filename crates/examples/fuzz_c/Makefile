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

# Generation only (no clang needed): the production FSM source under the exact
# name the skeleton's #include expects, plus the libFuzzer skeleton from the
# `test` command (written next to its main artifact).
generate: $(BUILD_DIR)/fuzz_target.c $(BUILD_DIR)/rfs_generated_fsm.c

$(BUILD_DIR)/rfs_generated_fsm.c: $(SPEC)
	mkdir -p $(BUILD_DIR)
	$(RFS) codegen c production $(SPEC) --out $@

$(BUILD_DIR)/fuzz_target.c: $(SPEC)
	mkdir -p $(BUILD_DIR)
	$(RFS) codegen c test $(SPEC) --out $(BUILD_DIR)/spec_test.c
	rm -f $(BUILD_DIR)/spec_test.c

# The skeleton #includes "rfs_generated_fsm.c" relative to its own directory,
# so both files live in build/ and no -I is needed.
fuzz_target: $(BUILD_DIR)/fuzz_target.c $(BUILD_DIR)/rfs_generated_fsm.c
	$(CLANG) -O1 -fsanitize=fuzzer,address -o $@ $(BUILD_DIR)/fuzz_target.c

# Bounded smoke run (a real campaign is user work, not a build target).
run: fuzz_target
	./fuzz_target -max_total_time=10

clean:
	rm -rf $(BUILD_DIR) fuzz_target
