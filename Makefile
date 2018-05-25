#!make

# SCCACHE can be 'local' or 'off' (default)
# local will cache in ~/.cache/sccache
SCCACHE ?= off
SCCACHE_CMD ?= ~/.cargo/bin/sccache

CARGO_CMD ?= $(if $(filter off,$(SCCACHE)),,RUSTC_WRAPPER=$(SCCACHE_CMD) )cargo

# Default target
all: test examples bench

CARGO_TEST_FLAGS ?=
CARGO_BUILD_FLAGS ?=

# 'test' is a friendly alias for 'unit_test'
test:
	$(CARGO_CMD) test

examples:
	$(CARGO_CMD) build --examples

bench:
	$(CARGO_CMD) +nightly bench --features "bench"

clean:
	$(CARGO_CMD) clean

publish: test examples bench
	cargo publish

.PHONY: all build clean test examples bench publish

