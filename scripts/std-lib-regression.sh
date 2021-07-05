#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test for platform
platform=`uname -sp`
if [[ $platform != "Linux x86_64" ]]; then
  echo "Codegen script only works on Linux x86 platform"
  exit 0
fi

# Get RMC root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
RMC_DIR=$SCRIPT_DIR/..

# Log output
STD_LIB_LOG="/tmp/SizeAndAlignOfDstTest/log.txt"

# Use a unit test that requires mutex and cell
echo "Starting RMC codegen for the Rust standard library"
cd /tmp
if [ -d SizeAndAlignOfDstTest ]; then rm -rf SizeAndAlignOfDstTest; fi
cargo new SizeAndAlignOfDstTest
cd SizeAndAlignOfDstTest
cp $RMC_DIR/src/test/cbmc/SizeAndAlignOfDst/main_fail.rs src/main.rs 
rustup component add rust-src --toolchain nightly > /dev/null 2>&1
RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo +nightly build -Z build-std --target x86_64-unknown-linux-gnu 2> $STD_LIB_LOG

# For now, we expect a linker error, but no modules should fail with a compiler
# panic. 
#
# With https://github.com/model-checking/rmc/issues/109, this check can be
# removed to just allow the success of the previous line to determine the 
# success of this script (with no $STD_LIB_LOG needed)
RESULT=$?
if grep -q "error: internal compiler error: unexpected panic" $STD_LIB_LOG; then
  echo "Panic on building standard library"
  cat $STD_LIB_LOG
  exit 1
else 
echo "Successful RMC codegen for the Rust standard library"
fi
