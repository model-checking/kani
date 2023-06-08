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
else
  echo
  echo "Firecracker codegen regression only works on Linux x86 platform, skipping..."
  echo
  exit 0
fi

# Get Kani root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$SCRIPT_DIR/..

echo
echo "Starting Firecracker codegen regression..."
echo

# At the moment, we only test codegen for the virtio module
cd $KANI_DIR/firecracker/src/devices/src/virtio/

# Clean first
cargo clean

export KANI_LOG=error
export RUSTC_LOG=error
export RUST_BACKTRACE=1

# Compile rust to iRep
RUST_FLAGS=(
    "--kani-compiler"
    "-Cpanic=abort"
    "-Zalways-encode-mir"
    "-Cllvm-args=--backend=c_prover"
    "-Cllvm-args=--ignore-global-asm"
    "-Cllvm-args=--reachability=pub_fns"
    "--sysroot=${KANI_DIR}/target/kani"
)
export RUSTFLAGS="${RUST_FLAGS[@]}"
export RUSTC="$KANI_DIR/target/kani/bin/kani-compiler"
$WRAPPER cargo build --verbose --lib --target $TARGET

echo
echo "Finished Firecracker codegen regression successfully..."
echo
