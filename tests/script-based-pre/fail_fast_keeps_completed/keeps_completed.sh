#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Regression test for https://github.com/model-checking/kani/issues/4729:
# under `--fail-fast`, a harness that already completed must not be dropped from
# the final summary when a later harness fails and aborts the run.
#
# `sort_harnesses_by_loc` processes later-appearing harnesses first, so in the
# default sequential run `z_passes` completes before `a_fails` aborts the run.
# Previously the completed pass was discarded and the summary reported
# "0 successfully verified harnesses ... 1 total"; it must now count the pass.
#
# The `--export-json` half of this regression lives in tests/json-handler/fail-fast/,
# with the rest of the export coverage.

set +e

OUT=$(kani fixture.rs --fail-fast 2>&1)
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

echo "SUCCESS: --fail-fast retains already-completed harness results"
