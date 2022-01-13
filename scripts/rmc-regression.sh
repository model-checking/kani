#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ -z $RMC_REGRESSION_KEEP_GOING ]]; then
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
RMC_DIR=$SCRIPT_DIR/..

# Required dependencies
check-cbmc-version.py --major 5 --minor 48
check-cbmc-viewer-version.py --major 2 --minor 5

# Formatting check
./x.py fmt --check

# Build RMC compiler and RMC library
(cd "${RMC_DIR}/src/rmc-compiler"; cargo build)

# Unit tests
(cd src/rmc-compiler/cbmc; cargo test)
(cd src/rmc-compiler; cargo test)

# Build tool for linking RMC pointer restrictions
cargo build --release --manifest-path src/tools/rmc-link-restrictions/Cargo.toml 

# Build compiletest
(cd "${RMC_DIR}/src/tools/compiletest"; cargo build --release)

# Declare testing suite information (suite and mode)
TESTS=(
    "rmc rmc"
    "firecracker rmc"
    "prusti rmc"
    "smack rmc"
    "expected expected"
    "cargo-rmc cargo-rmc"
    "rmc-docs rmc"
    "rmc-fixme rmc-fixme"
)

# Extract testing suite information and run compiletest
for testp in "${TESTS[@]}"; do
  testl=($testp)
  suite=${testl[0]}
  mode=${testl[1]}
  echo "Check compiletest suite=$suite mode=$mode ($TARGET -> $TARGET)"
  ./target/release/compiletest --rmc-dir-path scripts --src-base src/test/$suite --build-base build/$TARGET/test/$suite --stage-id stage1-$TARGET --suite $suite --mode $mode --target $TARGET --host $TARGET --quiet --channel dev
done

# Check codegen for the standard library
time "$SCRIPT_DIR"/std-lib-regression.sh

# We rarely benefit from re-using build artifacts in the firecracker test,
# and we often end up with incompatible leftover artifacts:
# "error[E0514]: found crate `serde_derive` compiled by an incompatible version of rustc"
# So if we're calling the full regression suite, wipe out old artifacts.
if [ -d "$RMC_DIR/firecracker/build" ]; then
  rm -rf "$RMC_DIR/firecracker/build"
fi

# Check codegen of firecracker
time "$SCRIPT_DIR"/codegen-firecracker.sh

# Check that we can use RMC on crates with a diamond dependency graph,
# with two different versions of the same crate.
#
#         dependency1
#        /           \ v0.1.0
#   main             dependency3
#        \           / v0.1.1
#         dependency2
time "$RMC_DIR"/src/test/rmc-dependency-test/diamond-dependency/run-dependency-test.sh

# Check that we don't have type mismatches across different crates
time "$RMC_DIR"/src/test/rmc-multicrate/type-mismatch/run-mismatch-test.sh

echo
echo "All RMC regression tests completed successfully."
echo
