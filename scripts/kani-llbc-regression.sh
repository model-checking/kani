#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ -z $KANI_REGRESSION_KEEP_GOING ]]; then
  set -o errexit
fi
set -o pipefail
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export PATH=$SCRIPT_DIR:$PATH

# Formatting check
${SCRIPT_DIR}/kani-fmt.sh --check

# Build kani
cargo build-dev -- --features cprover --features llbc

# The LLBC backend is only built by this script, so its warnings and lints are checked here: CI's
# `-D warnings` build and clippy run cover the default (`cprover`) features only.
# The `-D warnings` build also covers Charon itself: it is a path dependency, and cargo only caps
# lints for registry and git dependencies, so a Charon pin that introduces a warning fails here.
# The clippy run lints `kani-compiler` only.
echo "--- Build and lint the LLBC backend with warnings denied"
RUSTFLAGS="-D warnings" cargo build --target-dir /tmp/kani_llbc_build_warnings --features llbc
cargo clippy -p kani-compiler --features llbc -- -D warnings

# Build compiletest and print configuration. We pick suite / mode combo so there's no test.
echo "--- Compiletest configuration"
cargo run -p compiletest --quiet -- --suite kani --mode cargo-kani --dry-run --verbose
echo "-----------------------------"

suite="llbc"
mode="expected"
echo "Check compiletest suite=$suite mode=$mode"
# `--require-success`: the LLBC tests only pin output, and Kani can print the expected LLBC and
# still fail afterwards (e.g. when Charon reports errors), which would otherwise go unnoticed.
cargo run -p compiletest --quiet -- --suite $suite --mode $mode \
    --quiet --no-fail-fast --require-success

echo
echo "All Kani llbc regression tests completed successfully."
echo
