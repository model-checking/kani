#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

TMP_DIR="tmp_dir"

rm -rf ${TMP_DIR}
cp -r sample_crate ${TMP_DIR}
pushd ${TMP_DIR} > /dev/null


echo "[TEST] Generate test..."
cargo kani --concrete-playback=inplace -Z concrete-playback

echo "[TEST] Run test..."
cargo kani playback -Z concrete-playback

# Cleanup
popd > /dev/null
rm -r ${TMP_DIR}
