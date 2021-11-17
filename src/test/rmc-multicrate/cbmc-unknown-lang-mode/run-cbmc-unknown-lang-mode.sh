#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

echo
echo "Starting CBMC unknown language mode test..."
echo

# Compile crates with RMC backend
cd $(dirname $0)
rm -rf /tmp/cbmc_lang_mode_test_build
cd unknown-lang-mode
RESULT="/tmp/cbmc_lang_mode_test_result.txt"

export RUSTC_LOG=error
export CARGO_TARGET_DIR=/tmp/type_mismatch_test_build
export RUST_BACKTRACE=1
cargo rmc --function test &> $RESULT

if ! grep -q "VERIFICATION SUCCESSFUL" $RESULT; then
  cat $RESULT
  echo
  echo "Failed unknown language mode test"
  echo
  exit 1
fi

echo
echo "Finished unknown language mode test successfully..."
echo
