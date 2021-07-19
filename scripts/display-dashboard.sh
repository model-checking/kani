#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
RUST_DIR=$SCRIPT_DIR/..
export PATH=$SCRIPT_DIR:$PATH

cargo build --manifest-path src/tools/dashboard/Cargo.toml
cargo run --manifest-path src/tools/dashboard/Cargo.toml
