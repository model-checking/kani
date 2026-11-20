#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Test that `--concrete-playback=inplace` adds the test to the source file when Kani runs inside
# a workspace member, where the compiler records the file relative to the workspace root.

set +e

TMP_DIR="tmp_dir"

rm -rf ${TMP_DIR}
cp -r sample_workspace ${TMP_DIR}
pushd ${TMP_DIR}/member > /dev/null

echo "[TEST] Generate test..."
cargo kani --concrete-playback=inplace -Z concrete-playback

echo "[TEST] Check source..."
if grep -q "fn kani_concrete_playback_harness" src/lib.rs; then
    echo "Test added to member/src/lib.rs"
fi

echo "[TEST] Run test..."
cargo kani playback -Z concrete-playback

# Cleanup
popd > /dev/null
rm -r ${TMP_DIR}
