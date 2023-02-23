#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Checks situations where running kani multiple times will not trigger a recompilation
# The cases we cover here are:
# - Exact same input being invoked 2x.
# - Different options that do not influence the compilation only the Kani flow.
# - Different options that do not influence the compilation only the CBMC execution.

MANIFEST=lib/Cargo.toml
OUT_DIR=target

# Ensure output folder is clean
rm -rf ${OUT_DIR}
mkdir -p ${OUT_DIR}

function check_result {
    local log_file="${OUT_DIR}/$1"
    # Check for occurrances of "Compiling" messages in the log files
    grep "Compiling" -H -c ${log_file}
    # Check which harnesses ran
    grep "Checking harness" -H ${log_file} || echo "${log_file}: No harness"
    # Check the verification summary
    grep "successfully verified harnesses" -H ${log_file} || echo "${log_file}: ok"
}

echo "Initial compilation"
cargo kani --manifest-path ${MANIFEST} --target-dir ${OUT_DIR} --only-codegen 2>&1 | tee ${OUT_DIR}/initial.log
check_result initial.log

echo "Re-execute the same command"
cargo kani --manifest-path ${MANIFEST} --target-dir ${OUT_DIR} --only-codegen 2>&1 | tee ${OUT_DIR}/same.log
check_result same.log

echo "Run with new arg that affects kani-driver workflow only"
cargo kani --manifest-path ${MANIFEST} --target-dir ${OUT_DIR} 2>&1 | tee ${OUT_DIR}/driver_opt.log
check_result driver_opt.log

echo "Run with a new cbmc option"
cargo kani --manifest-path ${MANIFEST} --target-dir ${OUT_DIR} --no-default-checks 2>&1 | tee ${OUT_DIR}/cbmc_opt.log
check_result cbmc_opt.log

# Try to leave a clean output folder at the end
rm -rf ${OUT_DIR}
