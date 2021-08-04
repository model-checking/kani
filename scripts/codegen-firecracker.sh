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

# At the moment, we only test codegen for the virtio module
git submodule update --init --recursive
cd $RMC_DIR/firecracker/src/devices/src/virtio/
RUST_BACKTRACE=1 RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu
