#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

OUT_DIR=bin/target

echo
echo "Starting output file check..."
echo


# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "cargo command not found. Please install Rust and Cargo."
    exit 1
fi

# Check if the directory containing cargo is in the PATH environment variable
if ! echo "$PATH" | grep -q "$(dirname "$(command -v cargo)")"; then
    echo "Directory containing cargo is not in PATH. Adding directory to PATH..."
    echo "export PATH=$PATH:$(dirname "$(command -v cargo)")" >> ~/.bashrc
    source ~/.bashrc
fi

echo "Running cargo test on the unit test ..."
echo

rm -f test_output.log

cd bin/

# Run cargo test on the unit test
RUSTFLAGS="--cfg=kani" cargo +nightly test 2>/dev/null > test_output.log

echo "Checking content"
echo

# Check if the test failed
if grep -q "test result: FAILED" test_output.log; then
    echo "Tests failed as expected"
else
    echo "Tests passed"
fi

rm -f test_output.log

cd ..

# Try to leave a clean output folder at the end
rm -rf ${OUT_DIR}

set -eu
