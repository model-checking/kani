#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Test that concrete playback -Z concrete-playback executes as expected
set -o nounset

RS_FILE="modified.rs"
cp original.rs ${RS_FILE}

echo "[TEST] Generate test..."
kani ${RS_FILE} -Z concrete-playback --concrete-playback=inplace -Z function-contracts --output-format terse

# Note that today one of the tests will succeed since the contract pre-conditions are not inserted by Kani.
# Hopefully this will change with https://github.com/model-checking/kani/issues/3326
echo "[TEST] Run test..."
kani playback -Z concrete-playback ${RS_FILE} -- kani_concrete_playback

# Cleanup
rm ${RS_FILE}
rm kani_concrete_playback