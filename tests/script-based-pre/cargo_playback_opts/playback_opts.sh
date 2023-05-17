#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

pushd sample_crate > /dev/null
cargo clean

echo "[TEST] Only codegen test..."
cargo kani playback -Z concrete-playback --test kani_concrete_playback --only-codegen

echo "[TEST] Only codegen test..."
output=$(cargo kani playback -Z concrete-playback --test kani_concrete_playback --only-codegen --message-format=json)
executable=$(echo ${output} | jq 'select(.reason == "compiler-artifact") | .executable')

echo "[TEST] Executable"
echo ${executable}

cargo clean
popd > /dev/null
