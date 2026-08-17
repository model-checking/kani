#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Regression test for https://github.com/model-checking/kani/issues/4729:
# under `--fail-fast`, a harness that already completed must not be dropped from
# the final summary or the `--export-json` file when a later harness fails and
# aborts the run.
#
# `sort_harnesses_by_loc` processes later-appearing harnesses first, so in the
# default sequential run `z_passes` completes before `a_fails` aborts the run.
# Previously the completed pass was discarded and the summary reported
# "0 successfully verified harnesses ... 1 total"; it must now count the pass.
# The export derives from the same result vector (#4729 impact item 2), so the
# completed pass must appear there too.

set +e

EXPORT_FILE="fail_fast_export.json"
trap 'rm -f "${EXPORT_FILE}"' EXIT

OUT=$(kani fixture.rs --fail-fast -Z unstable-options --export-json "${EXPORT_FILE}" 2>&1)
CODE=$?

# The run must still fail overall (a harness failed).
if [[ ${CODE} -eq 0 ]]; then
    echo "FAIL: --fail-fast run exited 0 despite a failing harness"
    echo "${OUT}"
    exit 1
fi

# The already-completed passing harness must be retained in the summary.
if ! grep -q "1 successfully verified harnesses, 1 failures, 2 total" <<< "${OUT}"; then
    echo "FAIL: a completed harness was dropped from the --fail-fast summary; got:"
    grep -E "successfully verified|total" <<< "${OUT}"
    exit 1
fi

# The export must retain it as well: counters and per-harness results.
if [[ ! -f "${EXPORT_FILE}" ]]; then
    echo "FAIL: --export-json produced no file on a --fail-fast run"
    exit 1
fi

python3 - "${EXPORT_FILE}" << 'EOF'
import json
import sys

with open(sys.argv[1]) as f:
    data = json.load(f)

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


summary = data['verification_results']['summary']
for field, want in [('executed', 2), ('successful', 1), ('failed', 1)]:
    check(summary[field] == want,
          f"summary.{field} should be {want}, got {summary[field]}")

results = {r.get('harness_id'): r.get('status')
           for r in data['verification_results']['results']}
check(results.get('z_passes') == 'Success',
      f"z_passes should be Success in results, got {results.get('z_passes')}")
check(results.get('a_fails') == 'Failure',
      f"a_fails should be Failure in results, got {results.get('a_fails')}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)
EOF
if [[ $? -ne 0 ]]; then
    echo "FAIL: the export dropped or mislabeled a completed harness"
    exit 1
fi

echo "SUCCESS: --fail-fast retains already-completed harness results"
