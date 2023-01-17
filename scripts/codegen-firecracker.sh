#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Test for platform
PLATFORM=$(uname -sp)
if [[ $PLATFORM != "Linux x86_64" ]]; then
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
export KANI_LOG=error
export RUSTC_LOG=error
export RUST_BACKTRACE=1
# Use cargo assess since this is now our default way of assessing Kani suitability to verify a crate.
cargo kani --enable-unstable --only-codegen assess

echo
echo "Finished Firecracker codegen regression successfully..."
echo
