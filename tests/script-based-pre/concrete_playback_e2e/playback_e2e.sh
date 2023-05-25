#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

OUT_DIR=tmp_sample_crate

# Ensure output folder is clean
rm -rf ${OUT_DIR}

# Move the original source to the output folder since it will be modified
cp -r sample_crate ${OUT_DIR}
pushd $OUT_DIR

echo "Run verification..."
cargo kani

echo "Run ok test..."
cargo kani playback -Z concrete-playback -- any_is_ok

echo "Run error test..."
cargo kani playback -Z concrete-playback -- any_is_err

echo "Run all tests..."
cargo kani playback -Z concrete-playback -- concrete_playback

popd
rm -rf ${OUT_DIR}