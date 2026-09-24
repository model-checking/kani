#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test the `kani_lib!(alloc)` definitions in the `verify-std` flow.
# 1. Make a copy of the rust standard library.
# 2. Inject `kani_lib!` into `core` and `alloc`, with a proof and two functions in `alloc`.
# 3. Run autoharness over those functions; it runs the proof too.

set +e

TMP_DIR="tmp_dir"

rm -rf ${TMP_DIR}
mkdir ${TMP_DIR}

echo "[TEST] Copy standard library from the current toolchain"
SYSROOT=$(rustc --print sysroot)
STD_PATH="${SYSROOT}/lib/rustlib/src/rust/library"
cp -r "${STD_PATH}" "${TMP_DIR}"

echo "[TEST] Modify library"
printf '\n#[cfg(kani)]\nkani_core::kani_lib!(core);\n' >> ${TMP_DIR}/library/core/src/lib.rs
cat verify_alloc.rs >> ${TMP_DIR}/library/alloc/src/lib.rs
# Note: Prepending with sed doesn't work on MacOs the same way it does in linux.
cp ${TMP_DIR}/library/alloc/src/lib.rs ${TMP_DIR}/alloc_lib.rs
echo '#![cfg_attr(kani, feature(kani))]' > ${TMP_DIR}/library/alloc/src/lib.rs
cat ${TMP_DIR}/alloc_lib.rs >> ${TMP_DIR}/library/alloc/src/lib.rs

echo "[TEST] Run kani autoharness --std"
kani autoharness -Z autoharness -Z unstable-options --output-format=regular \
    --std "${TMP_DIR}/library" \
    --target-dir "${TMP_DIR}/target" \
    --include-pattern "verify_alloc::"

rm -r ${TMP_DIR}
