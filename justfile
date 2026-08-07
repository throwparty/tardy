mod nix "nix"

set shell := ["bash", "-eux", "-o", "pipefail", "-c"]

default: list

# List targets.
list:
    just --list

# Format the tree.
fmt:
    treefmt

# Lint the tree.
lint:
    just nix lint
    cargo clippy --all-targets --all-features -- -D warnings

# Test the tree.
test:
    cargo test

# Build the tree.
build:
    cargo build
