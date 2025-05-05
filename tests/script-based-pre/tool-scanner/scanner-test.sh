#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -e

# Run this inside a tmp folder in the current directory
OUT_DIR=output_dir
# Ensure output folder is clean
rm -rf ${OUT_DIR}
mkdir output_dir
# Move the original source to the output folder since it will be modified
cp test.rs ${OUT_DIR}
pushd $OUT_DIR

cargo run -p scanner test.rs --crate-type lib
wc -l *csv

# How to intepret these results:
# - If the function is "truly safe," i.e., there's no unsafe in its call graph, it will not show up in the output at all.
# - Otherwise, the count should match the rules described in scanner::call_graph::OverallStats::unsafe_distance.
echo "Unsafe Distance Results"
cat test_scan_unsafe_distance.csv

popd
rm -rf ${OUT_DIR}
