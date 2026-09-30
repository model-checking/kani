#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that harnesses[] is sorted by (crate_name, file, line, name), not by run order, and is the
# same under --jobs 4 and --jobs 1.

set -eu
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

ARTIFACTS=(jobs4.json jobs1.json)
cleanup() { rm -f "${ARTIFACTS[@]}"; }
trap cleanup EXIT

kani -Z export-json --output-format=terse --jobs 4 test.rs --export-json jobs4.json
kani -Z export-json --output-format=terse --jobs 1 test.rs --export-json jobs1.json

for f in "${ARTIFACTS[@]}"; do
    if [ ! -f "$f" ]; then
        echo "ERROR: JSON file $f was not created"
        exit 1
    fi
    python3 "$VALIDATOR" "$f" 2>&1 | tail -1
done

python3 << 'EOF'
import json
import sys

FILES = ["jobs4.json", "jobs1.json"]


def normalize_harness(h):
    h = dict(h)
    resources = dict(h.get("resources", {}))
    resources.pop("verification_time_s", None)
    h["resources"] = resources
    return h


def normalize_doc(doc):
    return [normalize_harness(h) for h in doc["harnesses"]]


docs = []
for f in FILES:
    with open(f) as fh:
        docs.append(json.load(fh))

failures = []

for f, doc in zip(FILES, docs):
    keys = [(h["crate_name"], h["file"], h["line"], h["name"]) for h in doc["harnesses"]]
    if keys != sorted(keys):
        failures.append(
            f"{f}: harnesses[] is not sorted by (crate_name, file, line, name): {keys}"
        )
    if len(doc["harnesses"]) != 4:
        failures.append(f"{f}: expected 4 harnesses, got {len(doc['harnesses'])}")

normalized = [normalize_doc(doc) for doc in docs]
reference = normalized[0]
for f, doc in zip(FILES[1:], normalized[1:]):
    if doc != reference:
        failures.append(
            f"{f}: harnesses[] differs from {FILES[0]} after excluding "
            f"resources.verification_time_s (the volatile fields) -- "
            f"ordering is NOT stable under --jobs N"
        )

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

names = [h["name"] for h in docs[0]["harnesses"]]
print(
    "jobs-ordering-stable: harnesses[] "
    f"({names}) is sorted and identical across --jobs 4 and --jobs 1"
)
EOF
