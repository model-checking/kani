#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that `kani autoharness --export-json` marks automatic harnesses and bounded arguments.

set -eu
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

OUTPUT_FILE="autoharness_output.json"
trap 'rm -f "$OUTPUT_FILE"' EXIT

kani autoharness -Z autoharness -Z export-json --bounded-arguments \
    "$SCRIPT_DIR/fixture.rs" --export-json "$OUTPUT_FILE"

if [ ! -f "$OUTPUT_FILE" ]; then
    echo "ERROR: JSON file $OUTPUT_FILE was not created"
    exit 1
fi
python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1

OUTPUT_FILE="$OUTPUT_FILE" python3 << 'PYEOF'
import json
import os
import sys

with open(os.environ["OUTPUT_FILE"]) as f:
    data = json.load(f)

# name -> (is_automatically_generated, is_bounded)
EXPECTED = {
    "packet_len": (True, True),
    "first_byte": (True, False),
    "add_one": (True, False),
    "manual_harness": (False, False),
}
actual = {h["name"].rsplit("::", 1)[-1]: h for h in data["harnesses"]}

failures = []
if len(data["harnesses"]) != len(EXPECTED) or set(actual) != set(EXPECTED):
    failures.append(f"harness set should be exactly {sorted(EXPECTED)}, got {sorted(actual)}")
for name, (generated, bounded) in EXPECTED.items():
    h = actual.get(name)
    if h is None:
        continue
    if h["is_automatically_generated"] is not generated:
        failures.append(f"{name}: is_automatically_generated should be {generated}, "
                        f"got {h['is_automatically_generated']!r}")
    if h["is_bounded"] is not bounded:
        failures.append(f"{name}: is_bounded should be {bounded}, got {h['is_bounded']!r}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print("autoharness-export: is_automatically_generated and is_bounded match the harness kinds "
      "(BoundedArbitrary argument bounded, unbounded slice argument and manual harness not)")
PYEOF

echo "All autoharness-export checks passed!"
