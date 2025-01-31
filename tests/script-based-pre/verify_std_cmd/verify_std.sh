#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test `kani verify-std` subcommand.
# 1. Make a copy of the rust standard library.
# 2. Add a few Kani definitions to it + a few harnesses
# 3. Execute Kani

set +e

TMP_DIR="tmp_dir"

rm -rf ${TMP_DIR}
mkdir ${TMP_DIR}

# Create a custom standard library.
echo "[TEST] Copy standard library from the current toolchain"
SYSROOT=$(rustc --print sysroot)
STD_PATH="${SYSROOT}/lib/rustlib/src/rust/library"
cp -r "${STD_PATH}" "${TMP_DIR}"

# Insert a small harness in one of the standard library modules.
CORE_CODE=$(cat verify_core.rs)

STD_CODE='
#[cfg(kani)]
#[cfg(not(uninit_checks))]
mod verify {
    use core::kani;
    #[kani::proof]
    fn check_non_zero() {
        let orig: u32 = kani::any();
        if let Some(val) = core::num::NonZeroU32::new(orig) {
            assert!(orig == val.into());
        } else {
            assert!(orig == 0);
        };
    }
}
'

echo "[TEST] Modify library"
echo "${CORE_CODE}" >> ${TMP_DIR}/library/core/src/lib.rs
echo "${STD_CODE}" >> ${TMP_DIR}/library/std/src/num.rs

# Note: Prepending with sed doesn't work on MacOs the same way it does in linux.
# sed -i '1s/^/#![cfg_attr(kani, feature(kani))]\n/' ${TMP_DIR}/library/std/src/lib.rs
cp ${TMP_DIR}/library/std/src/lib.rs ${TMP_DIR}/std_lib.rs
echo '#![cfg_attr(kani, feature(kani))]' > ${TMP_DIR}/library/std/src/lib.rs
cat ${TMP_DIR}/std_lib.rs >> ${TMP_DIR}/library/std/src/lib.rs

# Test that the command works inside the library folder and does not change
# the existing workspace
# See https://github.com/model-checking/kani/issues/3574
echo "[TEST] Only codegen inside library folder"
pushd "${TMP_DIR}/library" >& /dev/null
RUSTFLAGS="--cfg=uninit_checks" kani verify-std \
    -Z unstable-options \
    . \
    -Z function-contracts \
    -Z stubbing \
    -Z mem-predicates \
    --only-codegen
popd
# Grep should not find anything and exit status is 1.
grep -c kani ${TMP_DIR}/library/Cargo.toml \
    && echo "Unexpected kani crate inside Cargo.toml" \
    || echo "No kani crate inside Cargo.toml as expected"

echo "[TEST] Run kani verify-std"
export RUST_BACKTRACE=1
kani verify-std \
    -Z unstable-options \
    "${TMP_DIR}/library" \
    --target-dir "${TMP_DIR}/target" \
    -Z function-contracts \
    -Z stubbing \
    -Z mem-predicates

# Test that uninit-checks basic setup works on a no-core library
echo "[TEST] Run kani verify-std -Z uninit-checks"
RUSTFLAGS="--cfg=uninit_checks" kani verify-std \
    -Z unstable-options \
    "${TMP_DIR}/library" \
    --target-dir "${TMP_DIR}/target" \
    -Z function-contracts \
    -Z stubbing \
    -Z mem-predicates \
    -Z uninit-checks

# Cleanup
rm -r ${TMP_DIR}
