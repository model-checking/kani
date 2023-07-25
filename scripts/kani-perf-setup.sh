#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# This is just the setup stage of all performance benchmarks
# Other scripts should source this and invoke which setup they want to run

set -o pipefail
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$SCRIPT_DIR/..
BENCH_DIR="${KANI_DIR}/tests/benchmarks"

RUN_DIR="/tmp/kani_perf_run_$(date +%s)"

build_kani() {
# Build Kani using release mode.
  cargo build-dev -- --release
}

prep_bench_files() {
  mkdir -p "$( dirname "${RUN_DIR}")"
  cp -r "${BENCH_DIR}" "${RUN_DIR}"
  for overlay_dir in ${BENCH_DIR}/overlays/*/; do
    orig_dir=$(basename ${overlay_dir})
    echo "Copying overlays for $orig_dir"
    cp -r -v ${overlay_dir}* ${RUN_DIR}/${orig_dir}/
  done
}

cleanup_artifacts() {
  echo "Cleaning up..."
  rm -r ${RUN_DIR}
  rm "$@"
}

run_benchmarks() {
  local suite=$(basename "${SUITE}")
  local mode="cargo-kani-test"
  echo "Check compiletest suite=$suite mode=$mode"
  cargo run -p compiletest -- --suite $suite --mode $mode --no-fail-fast --report-time "$@"
}

print_result() {
  local exit_code=$1
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
  # Abort if there is any setup error.
  set -o errexit
  build_kani
  # Prepare for a verification first
  prep_bench_files

  # Now override expected files to just expect a successful build
  local expected_files=$(find ${RUN_DIR} -name "*expected")
  for expected_file in ${expected_files}; do
    echo "Compiling" > ${expected_file}
    echo "Finished" >> ${expected_file}
  done

  SUITE="${KANI_DIR}/tests/bench_build"
  rm "${SUITE}"
  ln -s "${RUN_DIR}" "${SUITE}"

  # Delay execution errors since we need to cleanup.
  set +o errexit
  run_benchmarks --kani-flag="--only-codegen"
  local exit_code=$?
  cleanup_artifacts "${SUITE}"
  print_result ${exit_code}
}

benchmark_verification() {
  # Abort if there is any setup error.
  set -o errexit
  build_kani
  prep_bench_files
  # Need to use "perf" here, or this change won't be backward compatible.
  SUITE="${KANI_DIR}/tests/perf"
  rm "${SUITE}"
  ln -s "${RUN_DIR}" "${SUITE}"

  # Delay execution errors since we need to cleanup.
  set +o errexit
  run_benchmarks
  local exit_code=$?
  cleanup_artifacts "${SUITE}"
  print_result ${exit_code}
}
