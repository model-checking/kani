#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# We're explicitly checking output rather than failing if the test fails
#set -eu

echo
echo "Starting type mismatch test..."
echo

# Compile crates with Kani backend
cd $(dirname $0)
rm -rf /tmp/type_mismatch_test_build
cd mismatch
RESULT="/tmp/dependency_test_result.txt"

# Disable warnings until https://github.com/model-checking/kani/issues/573 is fixed
export RUSTC_LOG=error
export CARGO_TARGET_DIR=/tmp/type_mismatch_test_build
export RUST_BACKTRACE=1
cargo kani &> $RESULT

if ! grep -q "VERIFICATION SUCCESSFUL" $RESULT; then
  cat $RESULT
  echo
  echo "Failed type mismatch test"
  echo
  exit 1
fi

echo
echo "Finished type mismatch test successfully..."
echo
