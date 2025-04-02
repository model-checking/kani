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
EXTRA_X_PY_BUILD_ARGS="${EXTRA_X_PY_BUILD_ARGS:-}"
KANI_DIR=$SCRIPT_DIR/..

# This variable forces an error when there is a mismatch on the expected
# descriptions from cbmc checks.
# TODO: We should add a more robust mechanism to detect python unexpected behavior.
export KANI_FAIL_ON_UNEXPECTED_DESCRIPTION="true"

# Gather dependencies version from top `kani-dependencies` file.
source "${KANI_DIR}/kani-dependencies"
# Sanity check dependencies values.
[[ "${CBMC_MAJOR}.${CBMC_MINOR}" == "${CBMC_VERSION%.*}" ]] || \
    (echo "Conflicting CBMC versions"; exit 1)
# Check if installed versions are correct.
check-cbmc-version.py --major ${CBMC_MAJOR} --minor ${CBMC_MINOR}
check_kissat_version.sh

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
