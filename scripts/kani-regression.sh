#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ -z $KANI_REGRESSION_KEEP_GOING ]]; then
  set -o errexit
fi
set -o pipefail
set -o nounset

# Test for platform
PLATFORM=$(uname -sp)
if [[ $PLATFORM == "Linux x86_64" ]]
then
  TARGET="x86_64-unknown-linux-gnu"
elif [[ $PLATFORM == "Darwin i386" ]]
then
  TARGET="x86_64-apple-darwin"
fi

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export PATH=$SCRIPT_DIR:$PATH
EXTRA_X_PY_BUILD_ARGS="${EXTRA_X_PY_BUILD_ARGS:-}"
KANI_DIR=$SCRIPT_DIR/..

# Required dependencies
check-cbmc-version.py --major 5 --minor 50
check-cbmc-viewer-version.py --major 2 --minor 5

# Formatting check
${SCRIPT_DIR}/kani-fmt.sh --check

# Build all packages in the workspace
cargo build

# Unit tests
cargo test -p cprover_bindings
cargo test -p kani-compiler

# Declare testing suite information (suite and mode)
TESTS=(
    "kani kani"
    "firecracker kani"
    "prusti kani"
    "smack kani"
    "expected expected"
    "cargo-kani cargo-kani"
    "kani-docs cargo-kani"
    "kani-fixme kani-fixme"
    "ui expected"
)

# Extract testing suite information and run compiletest
for testp in "${TESTS[@]}"; do
  testl=($testp)
  suite=${testl[0]}
  mode=${testl[1]}
  echo "Check compiletest suite=$suite mode=$mode ($TARGET -> $TARGET)"
  # Note: `cargo-kani` tests fail if we do not add `$(pwd)` to `--build-base`
  # Tracking issue: https://github.com/model-checking/kani/issues/755
  cargo run -p compiletest --  --kani-dir-path scripts --src-base tests/$suite \
                               --build-base $(pwd)/build/$TARGET/tests/$suite \
                               --stage-id stage1-$TARGET \
                               --suite $suite --mode $mode \
                               --target $TARGET --host $TARGET \
                               --quiet --channel dev
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

echo
echo "All Kani regression tests completed successfully."
echo
