#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Install Rust toolchain
curl https://sh.rustup.rs -sSf | sh -s -- -y \
  && source  ~/.cargo/env
