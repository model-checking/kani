#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

function check_playback {
  local OUTPUT=output.log
  cargo kani playback "${@}" >& $OUTPUT
  # Sort output so we can rely on the order.
  echo "$(grep "test verify::.* ok" $OUTPUT | sort)"
  echo
  echo "======= Raw Output ======="
  cat $OUTPUT
  echo "=========================="
  echo
  rm $OUTPUT
}

pushd sample_crate > /dev/null
cargo clean

echo "[TEST] Run all..."
check_playback -Z concrete-playback

echo "[TEST] Run lib..."
check_playback -Z concrete-playback --lib

echo "[TEST] Run bins..."
check_playback -Z concrete-playback --bins

echo "[TEST] Only foo tests..."
check_playback -Z concrete-playback --bin foo

cargo clean
popd > /dev/null
