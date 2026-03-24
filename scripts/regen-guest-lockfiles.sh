#!/usr/bin/env bash
# Regenerates Cargo.lock for every example guest project using the toolchain
# pinned in crates/airbender-build/src/constants.rs (DEFAULT_GUEST_TOOLCHAIN).
#
# Run this after bumping DEFAULT_GUEST_TOOLCHAIN to keep all guest lockfiles
# consistent with the reproducible build container.
#
# Usage:
#   ./scripts/regen-guest-lockfiles.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONSTANTS="$REPO_ROOT/crates/airbender-build/src/constants.rs"

TOOLCHAIN=$(grep 'DEFAULT_GUEST_TOOLCHAIN' "$CONSTANTS" | grep -oP '"nightly-[^"]+?"' | tr -d '"')
if [[ -z "$TOOLCHAIN" ]]; then
    echo "error: could not extract DEFAULT_GUEST_TOOLCHAIN from $CONSTANTS" >&2
    exit 1
fi

echo "toolchain: $TOOLCHAIN"

rustup toolchain install "$TOOLCHAIN" --no-self-update

for guest in "$REPO_ROOT"/examples/*/guest; do
    if [[ ! -f "$guest/Cargo.toml" ]]; then
        continue
    fi
    echo "regenerating $(realpath --relative-to="$REPO_ROOT" "$guest")/Cargo.lock"
    cargo "+$TOOLCHAIN" generate-lockfile --manifest-path "$guest/Cargo.toml"
done

echo "done"
