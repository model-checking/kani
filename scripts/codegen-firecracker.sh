#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
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

# Get RMC root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
RMC_DIR=$SCRIPT_DIR/..

echo
echo "Starting Firecracker codegen regression..."
echo

# At the moment, we only test codegen for the virtio module
cd $RMC_DIR/firecracker/src/devices/src/virtio/
RUST_BACKTRACE=1 RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu

echo
echo "Finished Firecracker codegen regression successfully..."
echo
