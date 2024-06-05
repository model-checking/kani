#!/usr/bin/env bash
# Copyright Kani Contributors
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
  # Temporarily disabled (in CI) to keeps CI times down
  # See https://github.com/model-checking/kani/issues/1578
  exit 0

  TARGET="x86_64-apple-darwin"
  # mac 'time' doesn't have -v
  WRAPPER=""
elif [[ $PLATFORM == "Darwin arm" ]]
then
  TARGET="aarch64-apple-darwin"
  # mac 'time' doesn't have -v
  WRAPPER=""
else
  echo
  echo "Std-Lib codegen regression only works on Linux or OSX x86 platforms, skipping..."
  echo
  exit 0
fi

ADD_TEST_TO_SOURCE=true
# Check if any arguments are provided
if [[ "$#" -gt 0 ]]; then
    # Check if the first argument is "no_test_add"
    if [[ "$1" == "--no_test_add" ]]; then
        ADD_TEST_TO_SOURCE=false
    fi
fi

# Get Kani root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$(dirname "$SCRIPT_DIR")

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

# Add some content to the rust file including an std function that is non-generic.
if $ADD_TEST_TO_SOURCE; then
  echo '
  pub fn main() {
      assert!("2021".parse::<u32>().unwrap() == 2021);
  }
  ' > src/lib.rs
fi


# Use same nightly toolchain used to build Kani
cp ${KANI_DIR}/rust-toolchain.toml .

echo "Starting cargo build with Kani"
export RUST_BACKTRACE=1
export RUSTC_LOG=error

RUST_FLAGS=(
    "-Zunstable-options"
    "-Zcrate-attr=feature(register_tool)"
    "-Zcrate-attr=register_tool(kanitool)"
    "--kani-compiler"
    "-Cllvm-args=--build-std"
    "-Cllvm-args=--ignore-global-asm"
    "-Cllvm-args=--goto-c"
    "-Cllvm-args=--reachability=harnesses"
    "-Cpanic=abort"
    "-Zalways-encode-mir"
    "--extern kani_core"
    "-L"
    "${KANI_DIR}/target/kani/lib"
    "--cfg=kani"
)
export RUSTFLAGS="${RUST_FLAGS[@]}"
export RUSTC_LOGS="info"

export RUSTC="$KANI_DIR/target/kani/bin/kani-compiler"
export __CARGO_TESTS_ONLY_SRC_ROOT="/home/ubuntu/rust-dev"
# Compile rust to iRep
$WRAPPER cargo build --verbose -Z build-std=core --lib --target $TARGET

echo
echo "Finished Kani codegen for the Rust standard library successfully..."
echo
