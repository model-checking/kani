#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
  && source  ~/.cargo/env
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
pushd ${SCRIPT_DIR}/../../
toolchain_version=$(grep 'channel = ' rust-toolchain.toml | cut -d '"' -f 2)
toolchain_components=$(grep 'components = ' rust-toolchain.toml | cut -d '=' -f 2- | sed 's/[",]//g' | sed 's/\[//' | sed 's/\]//')
rustup toolchain install $toolchain_version --component $toolchain_components
popd
