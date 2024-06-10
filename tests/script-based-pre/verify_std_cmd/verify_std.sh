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
CORE_CODE='
#[cfg(kani)]
#[unstable(feature = "kani", issue = "none")]
pub mod kani {
    pub use kani_core::proof;

    #[rustc_diagnostic_item = "KaniAnyRaw"]
    #[inline(never)]
    pub fn any_raw_inner<T>() -> T {
        loop {}
    }

    #[inline(never)]
    #[rustc_diagnostic_item = "KaniAssert"]
    pub const fn assert(cond: bool, msg: &'\''static str) {
        let _ = cond;
        let _ = msg;
    }

    #[kani_core::proof]
    #[kani_core::should_panic]
    fn check_panic() {
        let num: u8 = any_raw_inner();
        assert!(num != 0, "Found zero");
    }

    #[kani_core::proof_for_contract(obviously_true)]
    fn check_proof_contract() {
        obviously_true(true);
    }

    #[kani_core::requires(x == true)]
    fn obviously_true(x: bool) -> bool {
        x
    }

    #[kani_core::proof]
    fn check_success() {
        let orig: u8 = any_raw_inner();
        let mid = orig as i8;
        let new = mid as u8;
        assert!(orig == new, "Conversion round trip works");
    }

    pub fn assert_true(cond: bool) {
        assert!(cond)
    }

    pub fn assert_false(cond: bool) {
        assert!(!cond)
    }

    #[kani_core::proof]
    #[kani_core::stub(assert_true, assert_false)]
    fn check_stub() {
        // Check this is in fact asserting false.
        assert_true(false)
    }
}
'

STD_CODE='
#[cfg(kani)]
mod verify {
    use core::kani;
    #[kani::proof]
    fn check_non_zero() {
        let orig: u32 = kani::any_raw_inner();
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

echo "[TEST] Run kani verify-std"
export RUST_BACKTRACE=1
kani verify-std -Z unstable-options "${TMP_DIR}/library" --target-dir "${TMP_DIR}/target" -Z stubbing

# Cleanup
rm -r ${TMP_DIR}
