#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that `--exact` reports a filter matching no harness even when a sibling filter matches in two crates.

set -eu
set -o pipefail

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

mkdir -p "$WORK_DIR/crate_a/src" "$WORK_DIR/crate_b/src"

cat > "$WORK_DIR/Cargo.toml" <<'CARGO'
[workspace]
resolver = "2"
members = ["crate_a", "crate_b"]
CARGO

cat > "$WORK_DIR/crate_a/Cargo.toml" <<'CARGO'
[package]
name = "workspace_filter_diagnostic_a"
version = "0.1.0"
edition = "2021"
CARGO

cat > "$WORK_DIR/crate_a/src/lib.rs" <<'RUST'
#[cfg(kani)]
#[kani::proof]
fn common() {
    assert!(1 + 1 == 2);
}
RUST

cat > "$WORK_DIR/crate_b/Cargo.toml" <<'CARGO'
[package]
name = "workspace_filter_diagnostic_b"
version = "0.1.0"
edition = "2021"
CARGO

cat > "$WORK_DIR/crate_b/src/lib.rs" <<'RUST'
#[cfg(kani)]
#[kani::proof]
fn common() {
    assert!(2 + 2 == 4);
}
RUST

cd "$WORK_DIR"
OUTPUT_FILE="$WORK_DIR/out.json"
printf 'sentinel: not a real export document\n' > "$OUTPUT_FILE"
cp "$OUTPUT_FILE" "$WORK_DIR/sentinel.txt"

check_typo_rejected() {
    local label="$1"
    shift
    set +e
    OUT=$(cargo kani --workspace --exact --harness common --harness typo "$@" 2>&1)
    CODE=$?
    set -e

    if [[ ${CODE} -eq 0 ]]; then
        echo "FAIL ($label): an --exact filter with one unmatched name (typo) exited 0"
        echo "$OUT"
        exit 1
    fi
    if ! grep -qF "Failed to match the following harness(es):" <<< "$OUT" \
        || ! grep -qx "typo" <<< "$OUT"; then
        echo "FAIL ($label): expected the unmatched-filter diagnostic naming typo, got:"
        echo "$OUT"
        exit 1
    fi
}

check_typo_rejected "without --export-json"
check_typo_rejected "with --export-json" -Z export-json --export-json "$OUTPUT_FILE"

if ! cmp -s "$OUTPUT_FILE" "$WORK_DIR/sentinel.txt"; then
    echo "FAIL: a rejected filter set must leave a pre-existing --export-json target untouched"
    exit 1
fi

echo "SUCCESS: workspace filter diagnostic named typo, nonzero exit, export target untouched"
