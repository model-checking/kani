#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
# Don't use .cargo/env as that won't prepend .cargo/bin to the PATH when it's
# already somewhere in there
export PATH="$HOME/.cargo/bin:$PATH"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
pushd ${SCRIPT_DIR}/../../
rustup toolchain install
popd
