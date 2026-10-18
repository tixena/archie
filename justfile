# Default: list recipes.
default:
    @just --list

# Lint Rust (clippy + fmt check).
lint: lint-rust

lint-rust:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo fmt --all --check

build: build-rust

build-rust:
    cargo build

# Run the Rust test suite.
test: test-rust

test-rust:
    cargo test --workspace
    cargo test --workspace --all-features

# Full pipeline: lint everything, build everything, run tests.
all: lint build test
