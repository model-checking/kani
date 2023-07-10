#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

TMP_DIR="/tmp/build-rs"

rm -rf ${TMP_DIR}
cp -r . ${TMP_DIR}
pushd ${TMP_DIR} > /dev/null


echo "[TEST] Run verification..."
cargo kani --concrete-playback=inplace -Z concrete-playback

echo "[TEST] Run playback..."
cargo kani playback -Z concrete-playback --lib -- check_kani

echo "[TEST] Run test..."
cargo test --lib

# Cleanup
popd > /dev/null
rm -r ${TMP_DIR}
