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
CARGO_TARGET_DIR=/tmp/cbmc_lang_mode_test_build RUST_BACKTRACE=1 RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu

# Convert from JSON to Gotoc 
cd /tmp/cbmc_lang_mode_test_build/x86_64-unknown-linux-gnu/debug/deps
symtab2gb *.json --out a.out &> /dev/null

# Add the entry point and remove unused functions
goto-cc --function test *.out -o a.out &> /dev/null
goto-instrument --drop-unused-functions a.out b.out &> /dev/null

# Run the solver
RESULT="/tmp/cbmc_lang_mode_test_result.txt"
cbmc b.out &> $RESULT
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
