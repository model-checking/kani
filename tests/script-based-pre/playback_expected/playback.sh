#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# This will run Kani verification in every `src/*.rs` file followed by playback command.
# Expected output is generated from individual expected files.
set -o nounset

run() {
  input_rs=${1:?"Missing input file"}

  echo "[TEST] Generate test for $input_rs..."
  kani ${input_rs} \
    -Z concrete-playback --concrete-playback=inplace \
    -Z function-contracts -Z stubbing --output-format terse

  # Note that today one of the tests will succeed since the contract pre-conditions are not inserted by Kani.
  # Hopefully this will change with https://github.com/model-checking/kani/issues/3326
  echo "[TEST] Run test for $input_rs..."
  summary=$(kani playback -Z concrete-playback ${input_rs} -- kani_concrete_playback | grep "test result")
  echo "Result for $input_rs: $summary"
}

ROOT_DIR=$(git rev-parse --show-toplevel)
MODIFIED_DIR=modified
rm -rf $MODIFIED_DIR
mkdir $MODIFIED_DIR

for rs in src/*.rs
do
  [[ -e "${rs}" ]] || exit 1
  echo "Running ${rs}"
  cp "$rs" $MODIFIED_DIR
  pushd $MODIFIED_DIR
  run $(basename $rs)
  popd
done

  # Cleanup
rm -rf $MODIFIED_DIR