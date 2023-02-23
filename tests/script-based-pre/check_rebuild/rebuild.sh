#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Checks situations where running kani multiple times should trigger a new build
# The cases we cover here are:
# - Pass a new argument that affects compilation
# - Change the source code
# - Add a dependency
# Note: This should run in the folder where the script is.

OUT_DIR=target
MANIFEST=${OUT_DIR}/target_lib/Cargo.toml
LIB_SRC=${OUT_DIR}/target_lib/src/lib.rs

function check_result {
    local log_file="${OUT_DIR}/$1"
    # Check for occurrances of "Compiling" messages in the log files
    grep "Compiling" -H -c ${log_file}
    # Check which harnesses ran
    grep "Checking harness" -H ${log_file} || echo "${log_file}: No harness"
    # Check the verification summary
    grep "successfully verified harnesses" -H ${log_file} || echo "${log_file}: ok"
}

# Ensure output folder is clean
rm -rf ${OUT_DIR}
mkdir -p ${OUT_DIR}

# Copy the project so we don't make changes to the source code
cp -r ../target_lib ${OUT_DIR}

echo "Initial compilation"
cargo kani --manifest-path ${MANIFEST} --target-dir ${OUT_DIR} --no-assertion-reach-checks | tee ${OUT_DIR}/initial.log
check_result initial.log

echo "Run with a new argument that affects compilation"
cargo kani --manifest-path ${MANIFEST} --target-dir ${OUT_DIR} | tee ${OUT_DIR}/enable_checks.log
check_result enable_checks.log

echo "Run after change to the source code"
echo '
#[kani::proof]
fn noop_check() {}
' >> ${LIB_SRC}
cargo kani --manifest-path ${MANIFEST} --target-dir ${OUT_DIR} | tee ${OUT_DIR}/changed_src.log
check_result changed_src.log

echo "Run with new dependency"
cargo add new_dep --manifest-path ${MANIFEST} --path ../new_dep
cargo kani --manifest-path ${MANIFEST} --target-dir ${OUT_DIR} | tee ${OUT_DIR}/new_dep.log
check_result new_dep.log

# Try to leave a clean output folder at the end
# rm -rf ${OUT_DIR}
