#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Test that concrete playback -Z concrete-playback does not override std print
# functions

set -o nounset

function error() {
    echo $@
    # Cleanup
    rm ${RS_FILE}
    rm output.log
    exit 1
}

RS_FILE="modified.rs"
cp print_vars.rs ${RS_FILE}
export RUSTFLAGS="--edition 2021"

echo "[TEST] Generate test..."
kani ${RS_FILE} -Z concrete-playback --concrete-playback=inplace

echo "[TEST] Run test..."
kani playback -Z concrete-playback ${RS_FILE} 2>&1 | tee output.log

echo "------ Check output ----------"

set -e
while read -r line; do
    grep "${line}" output.log || error "Failed to find: \"${line}\""
done < expected

echo
echo "------ Output OK ----------"
echo

# Cleanup
rm ${RS_FILE}
rm output.log
