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

# Required dependencies
check-cbmc-version.py --major 5 --minor 61
check-cbmc-viewer-version.py --major 3 --minor 5

# Formatting check
${SCRIPT_DIR}/kani-fmt.sh --check

# Parser tests
PYTHONPATH=${SCRIPT_DIR} python3 -m unittest ${SCRIPT_DIR}/test_cbmc_json_parser.py

# Build all packages in the workspace
cargo build --workspace

# Unit tests
cargo test -p cprover_bindings
cargo test -p kani-compiler
cargo test -p kani-driver

# Check output files (--gen-c option)
echo "Check GotoC output file generation"
time "$KANI_DIR"/tests/output-files/check-output.sh

# Declare testing suite information (suite and mode)
TESTS=(
    "kani kani"
    "expected expected"
    "ui expected"
    "firecracker kani"
    "prusti kani"
    "smack kani"
    "cargo-kani cargo-kani"
    "cargo-ui cargo-kani"
    "kani-docs cargo-kani"
    "kani-fixme kani-fixme"
)

# Extract testing suite information and run compiletest
for testp in "${TESTS[@]}"; do
  testl=($testp)
  suite=${testl[0]}
  mode=${testl[1]}
  echo "Check compiletest suite=$suite mode=$mode"
  # Note: `cargo-kani` tests fail if we do not add `$(pwd)` to `--build-base`
  # Tracking issue: https://github.com/model-checking/kani/issues/755
  cargo run -p compiletest --quiet -- --suite $suite --mode $mode --quiet
done

# Check codegen for the standard library
time "$SCRIPT_DIR"/std-lib-regression.sh

# We rarely benefit from re-using build artifacts in the firecracker test,
# and we often end up with incompatible leftover artifacts:
# "error[E0514]: found crate `serde_derive` compiled by an incompatible version of rustc"
# So if we're calling the full regression suite, wipe out old artifacts.
if [ -d "$KANI_DIR/firecracker/build" ]; then
  rm -rf "$KANI_DIR/firecracker/build"
fi

# Check codegen of firecracker
time "$SCRIPT_DIR"/codegen-firecracker.sh

# Check that we can use Kani on crates with a diamond dependency graph,
# with two different versions of the same crate.
#
#         dependency1
#        /           \ v0.1.0
#   main             dependency3
#        \           / v0.1.1
#         dependency2
time "$KANI_DIR"/tests/kani-dependency-test/diamond-dependency/run-dependency-test.sh

# Check that documentation compiles.
cargo doc --workspace --no-deps --exclude std

echo
echo "All Kani regression tests completed successfully."
echo
