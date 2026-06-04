set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

build:
    cargo build

build-release:
    cargo build --release

fmt:
    cargo fmt

fmt-check:
    cargo fmt --check

lint:
    cargo clippy --all-targets --all-features -- -D warnings

[parallel]
test: test-pm test-fast

test-pm:
    ALUR_FAST_MODE=false cargo test --all-targets --all-features

test-fast:
    ALUR_FAST_MODE=true cargo test --all-targets --all-features

test-modes:
    ALUR_FAST_MODE=false cargo test --all-targets --all-features
    ALUR_FAST_MODE=true cargo test --all-targets --all-features

ci: fmt-check lint test

bench *args:
    node ./benchmark/run.mjs {{args}}

bench-profile:
    ./benchmark/profile.sh

[parallel]
tidy: fmt lint
