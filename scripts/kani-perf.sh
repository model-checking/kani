#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -o pipefail
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$SCRIPT_DIR/..

# Build Kani using release mode.
cargo build-dev -- --release

PERF_DIR="${KANI_DIR}/tests/perf"

# Copy expected files from overlay directories
to_delete=
for overlay_dir in ${PERF_DIR}/overlays/*/; do
  orig_dir=$(basename ${overlay_dir})
  echo "Copying overlays for $orig_dir"
  copy_output=$(cp -r -v ${overlay_dir}* ${PERF_DIR}/${orig_dir}/)
  copied_files=$(echo ${copy_output} | rev | cut -d' ' -f 1 | rev | tr -d "'")
  # Add to the list of files to delete
  to_delete="${to_delete} ${copied_files}"
done

suite="perf"
mode="cargo-kani-test"
# Bound each test's wall time so a runaway case (e.g. an OOM-prone harness)
# fails as a normal test failure with output, instead of triggering an
# unattributable runner OOM-kill / shutdown signal in CI.
# The cap is a guardrail against runaway/OOM, not a tight performance bound,
# so it must sit above the slowest legitimate test. The whole-crate s2n-quic
# tests run many heavy harnesses back-to-back; s2n-quic-core (34 harnesses,
# e.g. sync::spsc::alloc_test ~375s and inet::checksum::differential ~135s)
# takes ~1500s on CI. It passes cap-free on main, whose full suite is ~2650s,
# and this branch's suite matches that, so this is not a regression. Default
# 2400s (40 min) clears s2n-quic-core with margin while the realistic suite
# runtime (~45 min) stays well within the workflow step timeout.
# Override via KANI_PERF_TEST_TIMEOUT.
timeout_secs="${KANI_PERF_TEST_TIMEOUT:-2400}"
echo "Check compiletest suite=$suite mode=$mode timeout=${timeout_secs}s"
cargo run -p compiletest -- --suite $suite --mode $mode --no-fail-fast --timeout "$timeout_secs"
exit_code=$?

echo "Cleaning up..."
rm ${to_delete}

echo
if [ $exit_code -eq 0 ]; then
  echo "All Kani perf tests completed successfully."
else
  echo "***Kani perf tests failed."
fi
echo
exit $exit_code
