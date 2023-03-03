#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Checks situations where running kani multiple times will work as expected when
# the target crate is binary.
#
# The following checks should not trigger recompilation.
# - Exact same input being invoked a second time.
# - Different options that do not influence the compilation only the Kani flow.
# While the following should recompile the target.
# - Pass a new argument that affects compilation
# - Add a dependency
set -e
set -u

ORIG=bin
OUT_DIR=target
MANIFEST=${OUT_DIR}/${ORIG}/Cargo.toml

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

    # Check for occurrances of "Compiling" messages in the log files
    grep "Compiling" -H -c ${log_file} || echo "${log_file}: All fresh"
    # Check which harnesses ran
    grep "Checking harness" -H ${log_file} || echo "${log_file}: No harness"
    # Check the verification summary
    grep "successfully verified harnesses" -H ${log_file} || echo "${log_file}: ok"
}

# Ensure output folder is clean
rm -rf ${OUT_DIR}
mkdir -p ${OUT_DIR}
# Move the original source to the output folder since it will be modified
cp -r ${ORIG} ${OUT_DIR}

echo "Initial compilation"
check_kani --only-codegen initial.log

echo "Re-execute the same command"
check_kani --only-codegen same.log

echo "Run with new arg that affects kani-driver workflow only"
check_kani "" driver_opt.log

echo "Run with a new argument that affects compilation"
check_kani --no-assertion-reach-checks disable_checks.log

echo "Run with new dependency"
cargo new --lib ${OUT_DIR}/new_dep
cargo add new_dep --manifest-path ${MANIFEST} --path ${OUT_DIR}/new_dep
check_kani  --no-assertion-reach-checks new_dep.log

# Try to leave a clean output folder at the end
rm -rf ${OUT_DIR}
