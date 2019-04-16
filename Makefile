#!make

# Why put a makefile in a Cargo project?
# To collect the recipes that require several cargo invocations and / or special flags.
# Maybe I should write a tool for this...

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
	$(CARGO_CMD) test --features="skeptic"

examples:
	$(CARGO_CMD) build --examples

bench:
	$(CARGO_CMD) +nightly bench --features="bench"

lint:
	$(CARGO_CMD) +nightly clippy

clean:
	$(CARGO_CMD) clean

publish: test examples bench lint
	cargo publish

.PHONY: all build clean test examples bench publish

