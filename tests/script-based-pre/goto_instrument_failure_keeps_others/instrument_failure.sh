#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Regression test for https://github.com/model-checking/kani/issues/4966: when goto-instrument
# fails for one harness, Kani must report that harness as failed, go on with the other harnesses,
# print the summary, and exit with a failure code. A real goto-instrument out-of-memory failure
# is hard to trigger reliably, so `bin/goto-instrument` simulates one.
set +e

REAL_GOTO_INSTRUMENT=$(command -v goto-instrument)
export REAL_GOTO_INSTRUMENT
export PATH=$(pwd)/bin:$PATH

OUT=$(kani fixture.rs 2>&1)
CODE=$?

if [[ ${CODE} -eq 0 ]]; then
    echo "FAIL: the run exited 0 although goto-instrument failed for a harness"
    echo "${OUT}"
    exit 1
fi

for expected in \
    "Failed to prepare harness instrument_oom for verification" \
    "goto-instrument exited with status" \
    "Verification failed for - instrument_oom" \
    "1 successfully verified harnesses, 1 failures, 2 total"; do
    if ! grep -qF "${expected}" <<< "${OUT}"; then
        echo "FAIL: missing \"${expected}\" in the output:"
        echo "${OUT}"
        exit 1
    fi
done
echo "SUCCESS: a goto-instrument failure fails its harness only"

# `sort_harnesses_by_loc` checks later-appearing harnesses first, so `instrument_oom` runs before
# `passes`. Under `--fail-fast`, its failure must stop the run like a verification failure would.
OUT=$(kani fixture.rs --fail-fast 2>&1)
CODE=$?

if [[ ${CODE} -eq 0 ]]; then
    echo "FAIL: the --fail-fast run exited 0 although goto-instrument failed for a harness"
    echo "${OUT}"
    exit 1
fi

if ! grep -qF "0 successfully verified harnesses, 1 failures, 1 total" <<< "${OUT}"; then
    echo "FAIL: --fail-fast did not stop after the goto-instrument failure; got:"
    echo "${OUT}"
    exit 1
fi
echo "SUCCESS: --fail-fast stops after a goto-instrument failure"
