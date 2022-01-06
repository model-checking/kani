#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ -z $RMC_REGRESSION_KEEP_GOING ]]; then
  set -o errexit
fi
set -o pipefail
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export PATH=$SCRIPT_DIR:$PATH
EXTRA_X_PY_BUILD_ARGS="${EXTRA_X_PY_BUILD_ARGS:-}"
RMC_DIR=$SCRIPT_DIR/..

# Required dependencies
check-cbmc-version.py --major 5 --minor 48
check-cbmc-viewer-version.py --major 2 --minor 5

# Formatting check
./x.py fmt --check

# Build RMC and RMC library
./x.py build -i --stage 1 library/std ${EXTRA_X_PY_BUILD_ARGS}
./scripts/setup/build_rmc_lib.sh

# Standalone rmc tests, expected tests, and cargo tests
./x.py test -i --stage 1 rmc firecracker prusti smack expected cargo-rmc rmc-docs rmc-fixme
./x.py test -i --stage 0 compiler/cbmc

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
