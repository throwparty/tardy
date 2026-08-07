set shell := ["bash", "-eux", "-o", "pipefail", "-c"]

default: list

# List targets.
list:
    just --list --justfile {{source_directory()}}/mod.just

# Lint the tree.
lint:
    nix --extra-experimental-features 'nix-command flakes' flake show {{source_directory()}}
    nix --extra-experimental-features 'nix-command flakes' build {{source_directory()}}#tardy.cargoDeps --no-link

# Update Cargo hash.
update-hash:
    cd {{source_directory()}}/.. && .github/workflows/nix.update-hashes/update-cargo-hash.sh
