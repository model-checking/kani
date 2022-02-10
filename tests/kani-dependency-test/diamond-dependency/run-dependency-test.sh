#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# We're explicitly checking output rather than failing if the test fails
#set -eu

echo
echo "Starting Diamond Dependency Test..."
echo

# Test for platform
PLATFORM=$(uname -sp)
if [[ $PLATFORM == "Linux x86_64" ]]
then
  TARGET="x86_64-unknown-linux-gnu"
elif [[ $PLATFORM == "Darwin i386" ]]
then
  TARGET="x86_64-apple-darwin"
else
  echo
  echo "Test only works on Linux or OSX x86 platforms, skipping..."
  echo
  exit 0
fi

# Compile crates with Kani backend
cd $(dirname $0)
rm -rf build
RUST_BACKTRACE=1 cargo kani --target-dir build --only-codegen --keep-temps --verbose

# Convert from JSON to Gotoc
cd build/${TARGET}/debug/deps/
ls *.symtab.json | xargs symtab2gb

# Add the entry point and remove unused functions
goto-cc --function harness *.out -o a.out
goto-instrument --drop-unused-functions a.out b.out

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
