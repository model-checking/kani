#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -e
set -o pipefail
set -o nounset

cleanup()
{
  rm ${RS_FILE}
}
trap cleanup EXIT

RS_FILE="modified.rs"
cp array.rs ${RS_FILE}

echo "[TEST] Generate test..."
kani ${RS_FILE} -Z concrete-playback --concrete-playback=inplace

echo "[TEST] Run test..."
kani playback -Z concrete-playback ${RS_FILE}
