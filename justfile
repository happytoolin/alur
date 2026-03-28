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
    HNI_FAST=false cargo test --all-targets --all-features

test-fast:
    HNI_FAST=true cargo test --all-targets --all-features

test-all:
    node ./scripts/test-modes.mjs all

ci: fmt-check lint test

bench:
    ./benchmark/run.sh

bench-compare:
    ./benchmark/run.sh --track=compare

bench-fast:
    ./benchmark/run.sh --track=fast

bench-runtime:
    ./benchmark/run.sh --track=runtime

bench-direct:
    ./benchmark/run.sh --track=direct

bench-profile:
    ./benchmark/profile.sh

[parallel]
tidy: fmt lint
