#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Checks the two halves of `--log-file`: the terminal stays concise, and the
# detailed verification output actually lands in the file. A test that only
# asserts on stdout would pass even if the log were empty.

set -euo pipefail

log_dir=$(mktemp -d)
trap 'rm -rf "$log_dir"' EXIT
log=$log_dir/verification.log

echo "[run 1]"
kani fixture.rs --log-file "$log"

# The per-harness detail belongs in the file, not on the terminal. `kani` above
# is unredirected so the harness output would show up in `.expected` if it
# regressed to stdout; here we confirm the file received it.
for expected in \
    "Checking harness check_one" \
    "Checking harness check_two" \
    "VERIFICATION:- SUCCESSFUL"
do
    if ! grep -qF "$expected" "$log"; then
        echo "missing from log file: $expected"
        echo "--- log contents ---"
        cat "$log"
        exit 1
    fi
done

# Both harnesses, once each.
harnesses=$(grep -cF "Checking harness" "$log")
echo "harnesses logged: $harnesses"

echo "[run 2]"
# A second run must replace the log, not append to it: a reader grepping for a
# failure must not find a stale one from an earlier run.
kani fixture.rs --log-file "$log"
harnesses=$(grep -cF "Checking harness" "$log")
echo "harnesses logged after re-run: $harnesses"
