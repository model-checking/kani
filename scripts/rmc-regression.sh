#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ -z $RMC_REGRESSION_KEEP_GOING ]]; then
  set -o errexit
fi
set -o pipefail
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
RUST_DIR=$SCRIPT_DIR/..
export RMC_RUSTC=`find $RUST_DIR/build -name "rustc" -print | grep stage1`
export PATH=$SCRIPT_DIR:$PATH
EXTRA_X_PY_BUILD_ARGS="${EXTRA_X_PY_BUILD_ARGS:-}"

# Required dependencies
check-cbmc-version.py --major 5 --minor 30

# Formatting check
./x.py fmt --check

# Standalone rmc tests
pushd $RUST_DIR
./x.py build -i --stage 1 library/std ${EXTRA_X_PY_BUILD_ARGS}
cd rust-tests
for TEST_DIR in cbmc-reg smack-regressions prusti-regressions; do
    ./run.sh $TEST_DIR
done
./run.sh firecracker-like 2

# Standalone cargo-rmc tests
cd ../cargo-rmc-tests
for DIR in */; do
  ./run.py $DIR
done
popd

# run-make tests
./x.py test -i --stage 1 src/test/run-make --test-args gotoc
./x.py test -i --stage 1 src/test/cbmc
