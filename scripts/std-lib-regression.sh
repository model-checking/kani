#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Test for platform
PLATFORM=$(uname -sp)
if [[ $PLATFORM == "Linux x86_64" ]]
then
  TARGET="x86_64-unknown-linux-gnu"
  # 'env' necessary to avoid bash built-in 'time'
  WRAPPER="env time -v"
elif [[ $PLATFORM == "Darwin i386" ]]
then
  TARGET="x86_64-apple-darwin"
  # mac 'time' doesn't have -v
  WRAPPER=""
else
  echo
  echo "Std-Lib codegen regression only works on Linux or OSX x86 platforms, skipping..."
  echo
  exit 0
fi

# Get Kani root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$SCRIPT_DIR/..

echo
echo "Starting Kani codegen for the Rust standard library..."
echo

cd /tmp
if [ -d std_lib_test ]
then
    rm -rf std_lib_test
fi
cargo new std_lib_test --lib
cd std_lib_test

# Use same nightly toolchain used to build Kani
cp ${KANI_DIR}/rust-toolchain.toml .

echo "Starting cargo build with Kani"
export RUSTC_LOG=error
export KANIFLAGS="--goto-c"
export RUSTFLAGS="--kani-flags"
export RUSTC="${SCRIPT_DIR}/kani-rustc"
$WRAPPER cargo build --verbose -Z build-std --lib --target $TARGET

echo
echo "Finished Kani codegen for the Rust standard library successfully..."
echo
