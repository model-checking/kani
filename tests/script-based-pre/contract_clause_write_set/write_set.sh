#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

OUT=$(kani fixture.rs -Z function-contracts 2>&1)
CODE=$?
if [[ ${CODE} -ne 0 ]]; then
    echo "FAIL: verification failed"
    echo "${OUT}"
    exit 1
fi
if grep -q "not enough arguments" <<< "${OUT}"; then
    echo "FAIL: unexpected missing-argument warning"
    grep "not enough arguments" <<< "${OUT}"
    exit 1
fi

echo "SUCCESS: no missing-argument warning"
