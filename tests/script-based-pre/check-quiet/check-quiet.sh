#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Checks that `--quiet` produces no output and still reports the verdict through the exit code

set -euo pipefail

run_quiet() {
    QUIET_RC=0
    QUIET_BYTES=$(kani "$1" --quiet 2>&1 | wc -c) || QUIET_RC=$?
}

run_quiet assume.rs
if [[ ${QUIET_BYTES} -ne 0 ]]; then
    echo "error: \`--quiet\` produced output for a passing run"
    exit 1
fi
if [[ ${QUIET_RC} -ne 0 ]]; then
    echo "error: passing run under \`--quiet\` exited ${QUIET_RC}, expected 0"
    exit 1
fi
echo "success: passing run under \`--quiet\` produced no output and exited 0"

run_quiet fail.rs
if [[ ${QUIET_BYTES} -ne 0 ]]; then
    echo "error: \`--quiet\` produced output for a failing run"
    exit 1
fi
if [[ ${QUIET_RC} -ne 1 ]]; then
    echo "error: failing run under \`--quiet\` exited ${QUIET_RC}, expected 1"
    exit 1
fi
echo "success: failing run under \`--quiet\` produced no output and exited 1"
