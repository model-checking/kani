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
STD_LIB_LOG="/tmp/StdLibTest/log.txt"
echo "Starting RMC codegen for the Rust standard library"
cd /tmp
if [ -d StdLibTest ]; then rm -rf StdLibTest; fi
cargo new StdLibTest
cd StdLibTest

# Check that we have the nighly toolchain, which is required for -Z build-std
if ! rustup toolchain list | grep -q nightly; then
  echo "Installing nightly toolchain"
  rustup toolchain install nightly
fi

echo "Starting cargo build with RMC"
RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo +nightly build -Z build-std --target x86_64-unknown-linux-gnu &> $STD_LIB_LOG

# For now, we expect a linker error, but no modules should fail with a compiler
# panic. 
#
# With https://github.com/model-checking/rmc/issues/109, this check can be
# removed to just allow the success of the previous line to determine the 
# success of this script (with no $STD_LIB_LOG needed)

# TODO: this check is insufficient if the failure is before codegen
# https://github.com/model-checking/rmc/issues/375
if grep -q "error: internal compiler error: unexpected panic" $STD_LIB_LOG; then
  echo "Panic on building standard library"
  cat $STD_LIB_LOG
  exit 1
else 
echo "Successful RMC codegen for the Rust standard library"
fi