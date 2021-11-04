#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

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
if [ -d std_lib_test ]
then
    rm -rf std_lib_test
fi
cargo new std_lib_test --lib
cd std_lib_test

# Check that we have the nighly toolchain, which is required for -Z build-std
if ! rustup toolchain list | grep -q nightly; then
  echo "Installing nightly toolchain"
  rustup toolchain install nightly
fi

echo "Starting cargo build with RMC"
export RUSTC_LOG=error
export RUSTFLAGS=$(${SCRIPT_DIR}/rmc-rustc --rmc-flags)
export RUSTC=$(${SCRIPT_DIR}/rmc-rustc --rmc-path)
cargo +nightly build -Z build-std --lib --target $TARGET

echo
echo "Finished RMC codegen for the Rust standard library successfully..."
echo
