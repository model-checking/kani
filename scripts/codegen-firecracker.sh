#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test for platform
platform=`uname -sp`
if [[ $platform != "Linux x86_64" ]]; then
  echo "Codegen script only works on Linux x86 platform"
  exit 0
fi

# At the moment, we only test codegen for the virtio module
cd /tmp
git clone https://github.com/firecracker-microvm/firecracker.git
cd firecracker/src/devices/src/virtio/
RUST_BACKTRACE=1 RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu
