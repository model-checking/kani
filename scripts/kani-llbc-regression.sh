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

# Build compiletest and print configuration. We pick suite / mode combo so there's no test.
echo "--- Compiletest configuration"
cargo run -p compiletest --quiet -- --suite kani --mode cargo-kani --dry-run --verbose
echo "-----------------------------"

suite="llbc"
mode="expected"
echo "Check compiletest suite=$suite mode=$mode"
cargo run -p compiletest --quiet -- --suite $suite --mode $mode \
    --quiet --no-fail-fast

echo
echo "All Kani llbc regression tests completed successfully."
echo
