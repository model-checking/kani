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
echo '
pub fn main() {
    assert!("2021".parse::<u32>().unwrap() == 2021);
}
' > src/lib.rs

# Until we add support to this via our bundle, rebuild the kani library too.
echo "
kani = {path=\"${KANI_DIR}/library/kani\"}
" >> Cargo.toml

# Use same nightly toolchain used to build Kani
cp ${KANI_DIR}/rust-toolchain.toml .

echo "Starting cargo build with Kani"
export RUST_BACKTRACE=1
export RUSTC_LOG=error

RUST_FLAGS=(
    "--kani-compiler"
    "-Cpanic=abort"
    "-Zalways-encode-mir"
    "-Cllvm-args=--backend=c_prover"
    "-Cllvm-args=--ignore-global-asm"
    "-Cllvm-args=--reachability=pub_fns"
    "-Cllvm-args=--build-std"
)
export RUSTFLAGS="${RUST_FLAGS[@]}"
export RUSTC="$KANI_DIR/target/kani/bin/kani-compiler"
# Compile rust to iRep
$WRAPPER cargo build --verbose -Z build-std --lib --target $TARGET

echo
echo "Finished Kani codegen for the Rust standard library successfully..."
echo
