#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that JSON export through `cargo kani` reports the crate_name and the harness.

set -eu
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

# Scaffold the crate outside the repository so the build artifacts cannot pollute it.
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

mkdir -p "$WORK_DIR/src"
cat > "$WORK_DIR/Cargo.toml" <<'CARGO'
[package]
name = "json_export_cargo_test"
version = "0.1.0"
edition = "2021"
CARGO

cat > "$WORK_DIR/src/lib.rs" <<'RUST'
#[cfg(kani)]
#[kani::proof]
fn check_cargo_export() {
    let x: u8 = kani::any();
    kani::assume(x < 10);
    assert!(x < 20);
}
RUST

cd "$WORK_DIR"
OUTPUT_FILE="$WORK_DIR/cargo_output.json"

cargo kani -Z export-json --export-json "$OUTPUT_FILE"

if [ ! -f "$OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $OUTPUT_FILE was not created"
    exit 1
fi

python3 "$VALIDATOR" "$OUTPUT_FILE"

WORK_DIR="$WORK_DIR" python3 << 'EOF_PY'
import json
import os
import sys

with open(os.environ['WORK_DIR'] + '/cargo_output.json', 'r') as f:
    data = json.load(f)

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


names = [h.get('name') for h in data['harnesses']]
check(names == ['check_cargo_export'], f"unexpected harnesses: {names}")

crate_names = {h.get('crate_name') for h in data['harnesses']}
check(crate_names == {'json_export_cargo_test'},
      f"unexpected crate_name(s): {crate_names}")

summary = data['summary']
for field, want in [('total', 1), ('successful', 1), ('failed', 0)]:
    check(summary.get(field) == want,
          f"summary.{field} should be {want}, got {summary.get(field)}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("Cargo export reports the expected crate_name and harness")
EOF_PY
