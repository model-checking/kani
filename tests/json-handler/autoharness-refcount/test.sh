#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Autoharness harnesses whose generated values contain an `Rc`/`Arc` must be reported as
# bounded in their reference counts (#4752), without --bounded-arguments:
#   - export: `is_refcount_bounded` true exactly for those harnesses, `is_bounded` false;
#   - summary table: "(refcount)" exactly on those rows, plus the explanatory note;
#   - per-harness message: "all possible inputs" qualified exactly for those harnesses.
# Every cover in test.rs must be SATISFIED: an unsatisfiable cover alone does not fail
# verification, so this is checked here rather than left to the exit code.

set -eu
set -o pipefail

OUTPUT_FILE="autoharness_refcount.json"
LOG_FILE="autoharness_refcount.log"
trap 'rm -f "$OUTPUT_FILE" "$LOG_FILE"' EXIT

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"

# Sequential, so that per-harness messages are not interleaved with thread prefixes.
kani autoharness -Z autoharness -Z export-json --jobs 1 --output-format=terse test.rs \
    --export-json "$OUTPUT_FILE" > "$LOG_FILE" 2>&1 || {
    echo "ERROR: autoharness run failed (all functions should verify)"
    cat "$LOG_FILE"
    exit 1
}

python3 "$VALIDATOR" "$OUTPUT_FILE" 2>&1 | tail -1

python3 - "$OUTPUT_FILE" "$LOG_FILE" << 'EOF'
import json
import sys

with open(sys.argv[1]) as f:
    data = json.load(f)
with open(sys.argv[2]) as f:
    log = f.read()

MARKED = ["rc_arg", "rc_derivable", "arc_derivable", "arc_field", "rc_in_tuple", "arc_by_ref",
          "rc_in_box"]
UNMARKED = ["plain", "boxed", "phantom"]
# Functions whose covers must all be satisfied, with their number of covers.
COVERS = {"rc_arg": 1, "rc_derivable": 2, "arc_derivable": 2}
QUALIFIED = "all possible inputs (with finitely modeled Rc/Arc reference counts)..."
UNQUALIFIED = "all possible inputs..."

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


def short(name):
    return name.rsplit("::", 1)[-1]


harnesses = {short(h["name"]): h for h in data["harnesses"]}
check(sorted(harnesses) == sorted(MARKED + UNMARKED),
      f"unexpected harness set: {sorted(harnesses)}")

# Restrict row checks to the final summary; the initial function-selection table
# also contains one row per function but has no harness-kind/result columns.
_, sep, summary = log.partition("Autoharness Summary:")
check(bool(sep), "missing Autoharness Summary")
table_rows = [line for line in summary.splitlines() if line.startswith("|")]

for name in MARKED + UNMARKED:
    marked = name in MARKED
    h = harnesses.get(name)
    if h is None:
        continue
    check(h["is_automatically_generated"], f"{name}: not automatically generated")
    check(h["is_refcount_bounded"] is marked,
          f"{name}: is_refcount_bounded is {h['is_refcount_bounded']}, expected {marked}")
    check(h["is_bounded"] is False, f"{name}: is_bounded must not be set by Rc/Arc arguments")
    check(h["outcome"].get("verdict") == "SUCCESS", f"{name}: verdict {h['outcome']}")

    rows = [r for r in table_rows if f"| {name} " in r]
    check(len(rows) == 1, f"{name}: expected one summary row, found {rows}")
    if len(rows) == 1:
        check(("(refcount)" in rows[0]) is marked, f"{name}: summary row {rows[0]!r}")

    message = f"Autoharness: Checking function {name} against "
    expected = message + (QUALIFIED if marked else UNQUALIFIED)
    check(expected in log, f"{name}: missing message {expected!r}")

for name, n in COVERS.items():
    covers = harnesses.get(name, {}).get("covers", {})
    satisfied = covers.get("satisfied", [])
    check(covers.get("total") == n and len(satisfied) == n,
          f"{name}: expected {n} satisfied covers, got {covers}")

check(log.count('Note: harnesses marked "(refcount)"') == 1,
      "the (refcount) note must be printed exactly once")

if failures:
    print("FAILURES:")
    for f in failures:
        print(f"  - {f}")
    print("--- log ---")
    print(log)
    sys.exit(1)
print("All refcount reporting checks passed")
EOF
