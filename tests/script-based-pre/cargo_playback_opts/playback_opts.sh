#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

pushd sample_crate > /dev/null
cargo clean

echo "[TEST] Only codegen test..."
cargo kani playback -Z concrete-playback --only-codegen -- kani_concrete_playback

echo "[TEST] Only codegen test..."
output=$(cargo kani playback -Z concrete-playback --only-codegen --message-format=json -- kani_concrete_playback)

# Cargo may generate 2 artifacts, one for the library and one for tests.
executable=$(echo ${output} |
    jq 'select(.reason == "compiler-artifact") | select(.executable != null) | .executable')

echo "[TEST] Executable"
echo ${executable}

cargo clean
popd > /dev/null
