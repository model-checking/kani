#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Checks situations where running kani multiple times will not trigger a recompilation
# The cases we cover here are:
# - Exact same input being invoked 2x.
# - Different options that do not affect the compilation, only the Kani workflow.
# - Different options that do not affect the compilation, only the CBMC execution.

MANIFEST=lib/Cargo.toml
OUT_DIR=target

# Expects two arguments: "kani arguments" "output_file"
function check_kani {
    local args=$1
    local log_file="${OUT_DIR}/$2"
    # Run kani with the given arguments
    if [ -z "${args}" ]
    then
        cargo kani --manifest-path "${MANIFEST}" --target-dir "${OUT_DIR}" \
            2>&1 | tee "${log_file}"
    else
        cargo kani --manifest-path "${MANIFEST}" --target-dir "${OUT_DIR}" \
            "${args}" 2>&1 | tee "${log_file}"
    fi

    # Print information about the generated log file.
    # Check for occurrences of "Compiling" messages in the log files
    local compiled=$(grep -c "Compiling" ${log_file})
    echo "${log_file}:Compiled ${compiled} crates"

    # Check which harnesses were verified
    grep "Checking harness" -H ${log_file} || echo "${log_file}:No harness verified"

    # Check the verification summary
    grep "successfully verified harnesses" -H ${log_file} || true
}

# Ensure output folder is clean
rm -rf ${OUT_DIR}
mkdir -p ${OUT_DIR}

echo "Initial compilation"
check_kani --only-codegen initial.log

echo "Re-execute the same command"
check_kani --only-codegen same.log

echo "Run with new arg that affects kani-driver workflow only"
check_kani "" driver_opt.log

echo "Run with a new cbmc option"
check_kani --no-default-checks cbmc_opt.log

# Try to leave a clean output folder at the end
rm -rf ${OUT_DIR}
