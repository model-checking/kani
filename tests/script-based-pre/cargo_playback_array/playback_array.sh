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

cargo kani --concrete-playback inplace -Z concrete-playback
check_playback -Z concrete-playback

cargo clean
# Undo adding the concrete playback test
git restore src/lib.rs
popd > /dev/null
