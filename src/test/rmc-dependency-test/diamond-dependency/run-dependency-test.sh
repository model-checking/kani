#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# We're explicitly checking output rather than failing if the test fails
#set -eu

echo
echo "Starting Diamond Dependency Test..."
echo

TEMP_FOLD="/tmp/DependencyTest"
mkdir -p $TEMP_FOLD

DEP_LOG=$TEMP_FOLD/"log.txt"

# Compile crates with RMC backend
cd $(dirname $0)
rm -rf build
CARGO_TARGET_DIR=build RUST_BACKTRACE=1 RUSTFLAGS="-Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build > $DEP_LOG 2>&1

# Convert from JSON to Gotoc 
cd build/debug/deps/
ls *.json | xargs symtab2gb >> $DEP_LOG 2>&1

# Add the entry point and remove unused functions
goto-cc --function harness *.out -o a.out >> $DEP_LOG 2>&1
goto-instrument --drop-unused-functions a.out b.out >> $DEP_LOG 2>&1

# Run the solver
RESULT="/tmp/dependency_test_result.txt"
cbmc b.out &> $RESULT
if ! grep -q "VERIFICATION SUCCESSFUL" $RESULT; then
  cat $RESULT
  echo
  echo "Failed dependency test"
  echo
  exit 1
fi

echo
echo "Finished Diamond Dependency Test successfully..."
echo
