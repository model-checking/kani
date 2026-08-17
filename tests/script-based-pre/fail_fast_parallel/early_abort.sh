#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Parallel `--fail-fast` test, converted from a fixed `.expected` UI test when
# https://github.com/model-checking/kani/issues/4729 was fixed: completed
# results are now retained, so the summary counts depend on thread scheduling
# and cannot be pinned to a constant. This script asserts the properties that
# are stable: the run aborts early (fewer than all ten harnesses run), every
# harness that ran is counted, and the summary total equals the number of
# verdicts printed above it.

set +e

OUT=$(kani fixture.rs --fail-fast --jobs 4 --output-format=terse 2>&1)
CODE=$?

# A failing run must exit non-zero.
if [[ ${CODE} -eq 0 ]]; then
    echo "FAIL: --fail-fast run exited 0 despite failing harnesses"
    exit 1
fi

SUMMARY=$(grep "successfully verified harnesses" <<< "${OUT}")
read -r FAILURES TOTAL <<< "$(sed -n 's/.*Complete - \([0-9]*\) successfully verified harnesses, \([0-9]*\) failures, \([0-9]*\) total.*/\2 \3/p' <<< "${SUMMARY}")"

if [[ -z "${TOTAL}" || -z "${FAILURES}" ]]; then
    echo "FAIL: could not parse the summary line; got:"
    echo "${OUT}"
    exit 1
fi

# Every harness fails, so every counted harness must be a failure.
if [[ "${FAILURES}" -ne "${TOTAL}" ]]; then
    echo "FAIL: expected failures == total, got ${FAILURES} failures, ${TOTAL} total"
    exit 1
fi

# At least the failing harness itself is counted.
if [[ "${TOTAL}" -lt 1 ]]; then
    echo "FAIL: expected at least one counted harness, got ${TOTAL}"
    exit 1
fi

# The run must abort early: strictly fewer than all ten harnesses.
if [[ "${TOTAL}" -ge 10 ]]; then
    echo "FAIL: expected an early abort (< 10 harnesses), got ${TOTAL}"
    exit 1
fi

# The summary must count exactly the harnesses that printed a verdict above it
# (#4729's symptom was several verdicts above a "1 total" summary).
VERDICTS=$(grep -c "VERIFICATION:-" <<< "${OUT}")
if [[ "${VERDICTS}" -ne "${TOTAL}" ]]; then
    echo "FAIL: ${VERDICTS} verdicts printed but the summary counts ${TOTAL}"
    exit 1
fi

echo "SUCCESS: parallel --fail-fast aborts early and keeps completed results"
