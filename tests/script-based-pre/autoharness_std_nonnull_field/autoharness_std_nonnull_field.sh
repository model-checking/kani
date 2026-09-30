#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# In the `verify-std` flow `NonNull` can implement `Arbitrary` (verify-rust-std's impl casts an
# arbitrary integer). A struct with a `NonNull` field must still not be derived through it.
# 1. Make a copy of the rust standard library.
# 2. Inject `kani_lib!`, that impl and two functions into `core`.
# 3. List the automatic harnesses for those functions.

set +e

TMP_DIR="tmp_dir"

rm -rf ${TMP_DIR}
mkdir ${TMP_DIR}

echo "[TEST] Copy standard library from the current toolchain"
SYSROOT=$(rustc --print sysroot)
STD_PATH="${SYSROOT}/lib/rustlib/src/rust/library"
cp -r "${STD_PATH}" "${TMP_DIR}"

echo "[TEST] Modify library"
cat verify_nonnull_field.rs >> ${TMP_DIR}/library/core/src/lib.rs

echo "[TEST] Run kani autoharness --std --list"
# Capture the output first: piping the command straight into `grep` would hide its exit status.
list_output=$(kani autoharness -Z autoharness -Z unstable-options --list \
    --std "${TMP_DIR}/library" \
    --target-dir "${TMP_DIR}/target" \
    --include-pattern "verify_nonnull_field::" 2>&1)
echo "exit status: $?"
# The tables are padded to the longest name in `core`, so squeeze the padding.
echo "${list_output}" | grep -E "Kani generated automatic harnesses|verify_nonnull_field::" | tr -s ' '

rm -r ${TMP_DIR}
