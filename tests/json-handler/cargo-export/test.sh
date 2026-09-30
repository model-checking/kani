#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test JSON export through `cargo kani`. Every other test in this suite drives standalone
# `kani`, which leaves the Cargo argument handling untested and can never exercise the
# Cargo-only project metadata: `workspace_root` comes from Cargo metadata and is null for a
# standalone run, so only this test can tell whether it is reported correctly.

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

cargo kani -Z unstable-options --export-json "$OUTPUT_FILE"

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


project = data['project']

# The whole point of this test: a Cargo run must report the crate's workspace root, not the
# compiler output directory, and the two are different paths.
expected_root = os.path.realpath(os.environ['WORK_DIR'])
reported_root = project.get('workspace_root')
check(reported_root is not None, "workspace_root should not be null for a Cargo project")
if reported_root is not None:
    check(os.path.realpath(reported_root) == expected_root,
          f"workspace_root should be {expected_root}, got {reported_root}")

output_dir = project.get('output_dir')
check(output_dir is not None, "output_dir should not be null")
if output_dir is not None and reported_root is not None:
    check(os.path.realpath(output_dir) != os.path.realpath(reported_root),
          "output_dir should differ from workspace_root")

check(project.get('crate_name') == ['json_export_cargo_test'],
      f"unexpected crate_name: {project.get('crate_name')}")

names = [h.get('pretty_name') for h in data['harness_metadata']]
check(names == ['check_cargo_export'], f"unexpected harnesses: {names}")

summary = data['verification_results']['summary']
for field, want in [('total_harnesses', 1), ('executed', 1),
                    ('successful', 1), ('failed', 0)]:
    check(summary.get(field) == want,
          f"summary.{field} should be {want}, got {summary.get(field)}")

check(data['tools'].get('kani') is not None, "tools.kani should report a version")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("Cargo export reports the workspace root and the expected harness")
EOF_PY
