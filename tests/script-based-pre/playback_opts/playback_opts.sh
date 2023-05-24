#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Test that concrete playback -Z concrete-playback executes as expected
set -o pipefail
set -o nounset

RS_FILE="modified.rs"
cp original.rs ${RS_FILE}

echo "[TEST] Generate test..."
kani ${RS_FILE} -Z concrete-playback --concrete-playback=inplace

echo "[TEST] Only codegen test..."
kani playback -Z concrete-playback --test kani_concrete_playback --only-codegen ${RS_FILE}

echo "[TEST] Run test..."
kani playback -Z concrete-playback --test kani_concrete_playback ${RS_FILE}

echo "[TEST] Json format..."
kani playback -Z concrete-playback --test kani_concrete_playback ${RS_FILE} --only-codegen --message-format=json

# Cleanup
rm ${RS_FILE}
