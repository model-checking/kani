#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that a multi-crate `cargo kani --workspace` run writes one export keyed by (crate_name, name).

set -eu
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

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
name = "multi_crate_export_a"
version = "0.1.0"
edition = "2021"
CARGO

cat > "$WORK_DIR/crate_a/src/lib.rs" <<'RUST'
#[cfg(kani)]
#[kani::proof]
fn check_a() {
    let x: u8 = kani::any();
    kani::assume(x < 10);
    assert!(x < 20);
}

#[cfg(kani)]
#[kani::proof]
fn check_shared() {
    let x: u8 = kani::any();
    kani::assume(x < 5);
    assert!(x < 10);
}
RUST

cat > "$WORK_DIR/crate_b/Cargo.toml" <<'CARGO'
[package]
name = "multi_crate_export_b"
version = "0.1.0"
edition = "2021"
CARGO

cat > "$WORK_DIR/crate_b/src/lib.rs" <<'RUST'
#[cfg(kani)]
#[kani::proof]
fn check_b() {
    let x: u8 = kani::any();
    kani::assume(x < 20);
    assert!(x < 30);
}

#[cfg(kani)]
#[kani::proof]
fn check_shared() {
    let x: u8 = kani::any();
    kani::assume(x < 50);
    assert!(x < 100);
}
RUST

cd "$WORK_DIR"
OUTPUT_FILE="$WORK_DIR/multi_crate_output.json"

cargo kani --workspace -Z export-json --export-json "$OUTPUT_FILE"

if [ ! -f "$OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $OUTPUT_FILE was not created"
    exit 1
fi

python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1

WORK_DIR="$WORK_DIR" python3 << 'EOF_PY'
import json
import os
import sys

with open(os.environ['WORK_DIR'] + '/multi_crate_output.json', 'r') as f:
    data = json.load(f)

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


harnesses = data['harnesses']
by_key = {}
for h in harnesses:
    key = (h.get('crate_name'), h.get('name'))
    by_key.setdefault(key, []).append(h)

EXPECTED_KEYS = (
    ('multi_crate_export_a', 'check_a'),
    ('multi_crate_export_a', 'check_shared'),
    ('multi_crate_export_b', 'check_b'),
    ('multi_crate_export_b', 'check_shared'),
)
for key in EXPECTED_KEYS:
    entries = by_key.get(key, [])
    check(len(entries) == 1,
          f"{key} should appear exactly once, got {len(entries)} -- keys present: "
          f"{sorted(by_key)}")

check(len(harnesses) == len(EXPECTED_KEYS),
      f"expected {len(EXPECTED_KEYS)} harnesses total, got {len(harnesses)}")

summary = data['summary']
for field, want in [('total', 4), ('successful', 4), ('failed', 0)]:
    check(summary.get(field) == want,
          f"summary.{field} should be {want}, got {summary.get(field)}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("Multi-crate export: one file, both crates' harnesses present, distinguished by crate_name")
EOF_PY
