#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

OUT_DIR=sample_crate/target

echo
echo "Starting output file check..."
echo


# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "cargo command not found. Please install Rust and Cargo."
    exit 1
fi

echo "Running cargo test on the unit test ..."
echo

cd sample_crate/

output=$(grep 'channel = ' ../../../../rust-toolchain.toml | cut -d '"' -f 2)
echo "$output"

# Run cargo test on the unit test
RUSTFLAGS="--cfg=kani" cargo +${output} test 2>/dev/null

cd ..

# Try to leave a clean output folder at the end
rm -rf ${OUT_DIR}
