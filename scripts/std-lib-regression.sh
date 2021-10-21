#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Deliberately not enabling this, since we expect a failure currently and are failing based on 'grep' later
#set -eu

# Test for platform
PLATFORM=$(uname -sp)
if [[ $PLATFORM == "Linux x86_64" ]]; then
  TARGET="x86_64-unknown-linux-gnu"
elif [[ $PLATFORM == "Darwin i386" ]]; then
  TARGET="x86_64-apple-darwin"
else
  echo
  echo "Std-Lib codegen regression only works on Linux or OSX x86 platforms, skipping..."
  echo
  exit 0
fi

# Get RMC root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
RMC_DIR=$SCRIPT_DIR/..

echo
echo "Starting RMC codegen for the Rust standard library..."
echo

cd /tmp
if [ -d StdLibTest ]; then rm -rf StdLibTest; fi
cargo new StdLibTest
cd StdLibTest

# Check that we have the nighly toolchain, which is required for -Z build-std
if ! rustup toolchain list | grep -q nightly; then
  echo "Installing nightly toolchain"
  rustup toolchain install nightly
fi

TEMP_FOLD="/tmp/StdLibTest"
mkdir -p $TEMP_FOLD

STD_LIB_LOG=$TEMP_FOLD/"log.txt"

echo "Starting cargo build with RMC"
RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" \
  RUSTC=rmc-rustc cargo +nightly build -Z build-std --target $TARGET > $STD_LIB_LOG 2>&1

# For now, we expect a linker error, but no modules should fail with a compiler
# panic. 
#
# With https://github.com/model-checking/rmc/issues/109, this check can be
# removed to just allow the success of the previous line to determine the 
# success of this script (with no $STD_LIB_LOG needed)

# TODO: this check is insufficient if the failure is before codegen
# https://github.com/model-checking/rmc/issues/375
if grep -q "error: internal compiler error: unexpected panic" $STD_LIB_LOG; then
  echo
  echo "Panic on building standard library"
  echo
  exit 1
fi

echo
echo "Finished RMC codegen for the Rust standard library successfully..."
echo
