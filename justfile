#!just
# Sane make-like command runner: https://github.com/casey/just

# Default target
all: format test examples bench lint

# Unit and doc tests
test:
	cargo test --no-default-features --features="doc-comment"

examples:
	cargo build --examples

bench:
	cargo +nightly bench --features="bench"

format:
	cargo fmt

lint:
	cargo clippy

clean:
	cargo clean

# Build all and then publish to crates.io.
publish: all
	cargo publish
