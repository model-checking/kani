#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Checks the `--quiet` contract: it suppresses output but still reports the verdict through the
# process exit code. A passing run exits 0 with no output; a failing run exits non-zero with no
# output. The failing-run exit code is a regression test for issue #4745, where `--quiet` made a
# failing verification run exit 0.

# Note: no `set -e` -- we deliberately run a command expected to exit non-zero and inspect `$?`.
set -u

# Runs `kani <file> --quiet`, capturing combined output in QUIET_OUT and the exit code in QUIET_RC
# without aborting the script on a non-zero exit.
run_quiet() {
    QUIET_OUT=$(kani "$1" --quiet 2>&1) && QUIET_RC=0 || QUIET_RC=$?
}

# 1. A passing run: no output, exit 0.
run_quiet assume.rs
if [[ -n "${QUIET_OUT}" ]]; then
    echo "error: \`--quiet\` produced output for a passing run:"
    echo "${QUIET_OUT}"
    exit 1
fi
if [[ ${QUIET_RC} -ne 0 ]]; then
    echo "error: passing run under \`--quiet\` exited ${QUIET_RC}, expected 0"
    exit 1
fi
echo "success: passing run under \`--quiet\` produced no output and exited 0"

# 2. A failing run: no output, exit non-zero (regression test for #4745).
run_quiet fail.rs
if [[ -n "${QUIET_OUT}" ]]; then
    echo "error: \`--quiet\` produced output for a failing run:"
    echo "${QUIET_OUT}"
    exit 1
fi
if [[ ${QUIET_RC} -ne 1 ]]; then
    # A verification failure exits with exactly 1; requiring 1 (not just non-zero) keeps a crash
    # or signal death from passing this regression test. See #4745.
    echo "error: failing run under \`--quiet\` exited ${QUIET_RC}, expected 1 (see #4745)"
    exit 1
fi
echo "success: failing run under \`--quiet\` produced no output and exited non-zero"
