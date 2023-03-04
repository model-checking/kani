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

# Copy the project so we don't make changes to the source code
cp -r target_lib ${OUT_DIR}

echo "Initial compilation"
check_kani --no-assertion-reach-checks initial.log

echo "Run with a new argument that affects compilation"
check_kani "" enable_checks.log

echo "Run after change to the source code"
echo '
#[kani::proof]
fn noop_check() {}
' >> ${LIB_SRC}
check_kani "" changed_src.log

echo "Run with new dependency"
cargo new --lib ${OUT_DIR}/new_dep
cargo add new_dep --manifest-path ${MANIFEST} --path ${OUT_DIR}/new_dep
check_kani "" new_dep.log

# Try to leave a clean output folder at the end
rm -rf ${OUT_DIR}
