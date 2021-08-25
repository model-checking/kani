#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Compile crates with RMC backend
cd $(dirname $0)
rm -rf build
CARGO_TARGET_DIR=build RUST_BACKTRACE=1 RUSTFLAGS="-Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build

# Convert from JSON to Gotoc 
cd build/debug/deps/
ls *.json | xargs symtab2gb

# Add the entry point and remove unused functions
goto-cc --function harness *.out -o a.out 
goto-instrument --drop-unused-functions a.out b.out 

# Run the solver
RESULT="/tmp/dependency_test_result.txt"
cbmc b.out &> $RESULT
if grep -q "VERIFICATION SUCCESSFUL" $RESULT; then
  cat $RESULT
  echo "Successful dependency test"
  exit 0
else 
  cat $RESULT
  echo "Failed dependency test"
  exit 1
fi
