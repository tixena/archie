# Default: list recipes.
default:
    @just --list

# Lint Rust (clippy + fmt check).
lint: lint-rust

lint-rust:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo clippy -p archie --all-targets --no-default-features -- -D warnings
    cargo fmt --all --check

build: build-rust

build-rust:
    cargo build

# Run the Rust test suite.
test: test-rust

test-rust:
    cargo test --workspace
    cargo test --workspace --all-features

# Run the store tests against servers in containers this recipe starts and stops.
test-stores:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo clippy -p archie-store -p archie --all-targets --all-features --config 'build.rustflags=["--cfg", "store_tests"]' -- -D warnings
    name="archie-test-mongo-$$"
    docker run -d --rm --name "$name" -p 127.0.0.1::27017 mongo:latest >/dev/null
    trap 'docker stop "$name" >/dev/null' EXIT
    port=$(docker port "$name" 27017/tcp | head -1 | sed 's/.*://')
    until docker exec "$name" mongosh --quiet --eval 'db.runCommand({ ping: 1 }).ok' >/dev/null 2>&1; do sleep 1; done
    ARCHIE_TEST_MONGODB_URI="mongodb://127.0.0.1:$port" \
      cargo test -p archie-store -p archie --all-features --config 'build.rustflags=["--cfg", "store_tests"]'

# Full pipeline: lint everything, build everything, run tests.
all: lint build test test-stores
