#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Regression test for https://github.com/model-checking/kani/issues/4939:
# `kani autoharness` with a `--harness` filter must not panic when it also
# generates an automatic harness. In debug builds, `determine_targets` compared
# the harnesses kept by the compiler (which include all automatic harnesses)
# with a re-filter that skips automatic harnesses, and the two lists differed.

set +e

OUT=$(kani autoharness -Z autoharness fixture.rs --harness manual_selected 2>&1)
CODE=$?
if [[ ${CODE} -ne 0 ]]; then
    echo "FAIL: expected exit code 0, got ${CODE}:"
    echo "${OUT}"
    exit 1
fi
if grep -q "panicked" <<< "${OUT}"; then
    echo "FAIL: the driver panicked:"
    echo "${OUT}"
    exit 1
fi
# The filter selects one of the two manual harnesses.
if ! grep -q "Manual Harness Summary:" <<< "${OUT}" ||
    ! grep -q "Complete - 1 successfully verified harnesses, 0 failures, 1 total." <<< "${OUT}"; then
    echo "FAIL: expected exactly one verified manual harness, got:"
    echo "${OUT}"
    exit 1
fi
# The selected manual harness ran and the other one did not.
if ! grep -Fq "Checking harness manual_selected..." <<< "${OUT}"; then
    echo "FAIL: expected a check of manual_selected, got:"
    echo "${OUT}"
    exit 1
fi
if grep -Fq "manual_other" <<< "${OUT}"; then
    echo "FAIL: manual_other must not be selected, got:"
    echo "${OUT}"
    exit 1
fi
# The automatic harness for `add` is still verified.
if ! grep -q "Complete - 1 successfully verified functions, 0 failures, 1 total." <<< "${OUT}"; then
    echo "FAIL: expected one verified automatic harness, got:"
    echo "${OUT}"
    exit 1
fi

echo "SUCCESS: --harness filter keeps the automatic harness and selects one manual harness"
