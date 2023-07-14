#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# This is just the setup stage of all performance benchmarks
# Other scripts should source this and invoke which setup they want to run

set -o pipefail
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$SCRIPT_DIR/..
PERF_DIR="${KANI_DIR}/tests/perf"

RUN_DIR="/tmp/kani_perf_run_$(date +%s)"

build_kani() {
# Build Kani using release mode.
  cargo build-dev -- --release
}

prep_perf_files() {
  mkdir ${RUN_DIR} # We want to error out if the folder exists
  cp -r ${PERF_DIR} ${RUN_DIR}/perf
  for overlay_dir in ${PERF_DIR}/overlays/*/; do
    orig_dir=$(basename ${overlay_dir})
    echo "Copying overlays for $orig_dir"
    cp -r -v ${overlay_dir}* ${RUN_DIR}/perf/${orig_dir}/
  done
}

cleanup_perf() {
  echo "Cleaning up..."
  rm -r ${RUN_DIR}
  rm "$@"
}

run_benchmarks() {
  suite=$(basename "${SUITE}")
  mode="cargo-kani-test"
  echo "Check compiletest suite=$suite mode=$mode"
  cargo run -p compiletest -- --suite $suite --mode $mode --no-fail-fast --report-time "$@"
}

print_result() {
  exit_code=$1
  echo
  if [ $exit_code -eq 0 ]; then
    echo "All Kani perf tests completed successfully."
  else
    echo "***Kani perf tests failed."
  fi
  echo
  exit $exit_code
}

benchmark_build() {
  build_kani
  # Prepare for a verification first
  prep_perf_files
  SUITE="${KANI_DIR}/tests/bench_build"
  ln -s "${RUN_DIR}" "${SUITE}"

  # Now override expected files to just expect a successful build
  expected_files=$(find ${RUN_DIR} -name "*expected")
  for expected_file in ${expected_files}; do
    echo "Compiling" > ${expected_file}
    echo "Finished" >> ${expected_file}
  done

  run_benchmarks --kani-flag="--only-codegen"
  exit_code=$?
  cleanup_perf "${SUITE}"
  print_result ${exit_code}
}

benchmark_verification() {
  build_kani
  prep_perf_files
  SUITE="${KANI_DIR}/tests/bench_verification"
  ln -s "${RUN_DIR}" "${SUITE}"
  run_benchmarks
  exit_code=$?
  cleanup_perf
  print_result ${exit_code}
}
