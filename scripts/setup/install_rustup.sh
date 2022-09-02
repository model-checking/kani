#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Install Rust toolchain
curl https://sh.rustup.rs -sSf | sh -s -- -y \
  && source  ~/.cargo/env
