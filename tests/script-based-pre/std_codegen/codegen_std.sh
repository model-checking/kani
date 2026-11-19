#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test that we can codegen the entire standard library.
# 1. Make a copy of the rust standard library.
# 2. Add a few Kani definitions to it
# 3. Run Kani compiler

set -e
set -u

KANI_DIR=$(git rev-parse --show-toplevel)
TMP_DIR="tmp_dir"

rm -rf ${TMP_DIR}
mkdir ${TMP_DIR}

cp -r dummy ${TMP_DIR}

# Create a custom standard library.
echo "[TEST] Copy standard library from the current toolchain"
SYSROOT=$(rustc --print sysroot)
STD_PATH="${SYSROOT}/lib/rustlib/src/rust/library"
cp -r "${STD_PATH}" "${TMP_DIR}"

# Insert Kani definitions.
CORE_CODE='
#[cfg(kani)]
kani_core::kani_lib!(core);
'

echo "[TEST] Modify library"
echo "${CORE_CODE}" >> ${TMP_DIR}/library/core/src/lib.rs

# Note: Prepending with sed doesn't work on MacOs the same way it does in linux.
# sed -i '1s/^/#![cfg_attr(kani, feature(kani))]\n/' ${TMP_DIR}/library/std/src/lib.rs
cp ${TMP_DIR}/library/std/src/lib.rs ${TMP_DIR}/std_lib.rs
echo '#![cfg_attr(kani, feature(kani))]' > ${TMP_DIR}/library/std/src/lib.rs
cat ${TMP_DIR}/std_lib.rs >> ${TMP_DIR}/library/std/src/lib.rs

export RUST_BACKTRACE=1
export RUSTC_LOG=error
export __CARGO_TESTS_ONLY_SRC_ROOT=$(readlink -f ${TMP_DIR})/library
RUST_FLAGS=(
    "--kani-compiler"
    "-Cpanic=abort"
    "-Zalways-encode-mir"
    "-Cllvm-args=--backend=cprover"
    "-Cllvm-args=--ignore-global-asm"
    "-Cllvm-args=--reachability=pub_fns"
    "-L${KANI_DIR}/target/kani/no_core/lib"
    "--extern=kani_core"
    "--cfg=kani"
    "-Zcrate-attr=feature(register_tool)"
    "-Zcrate-attr=register_tool(kanitool)"
)
export RUSTFLAGS="${RUST_FLAGS[@]}"
export RUSTC="$KANI_DIR/target/kani/bin/kani-compiler"
export KANI_LOGS=kani_compiler::kani_middle=debug
TARGET=$(rustc -vV | awk '/^host/ { print $2 }')

pushd ${TMP_DIR}/dummy > /dev/null
# Compile the standard library to iRep
cargo build --verbose -Z build-std --lib --target ${TARGET}
popd > /dev/null

echo "------ Build succeeded -------"

# Cleanup
rm -r ${TMP_DIR}
