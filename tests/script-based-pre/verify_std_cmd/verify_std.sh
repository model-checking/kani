#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

TMP_DIR="tmp_dir"

rm -rf ${TMP_DIR}
mkdir ${TMP_DIR}

# Create a custom standard library.
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

    #[kani_core::proof]
    fn check_success() {
        let orig: u8 = any_raw_inner();
        let mid = orig as i8;
        let new = mid as u8;
        assert!(orig == new, "Conversion round trip works");
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
        let Some(val) = core::num::NonZeroU32::new(orig) else { assert!(orig == 0); return };
        assert!(orig == val.into());
    }
}
'

echo "${CORE_CODE}" >> ${TMP_DIR}/library/core/src/lib.rs
sed -i '1s/^/#![cfg_attr(kani, feature(kani))]\n/' ${TMP_DIR}/library/std/src/lib.rs
echo "${STD_CODE}" >> ${TMP_DIR}/library/std/src/num.rs

echo "[TEST] Run kani verify-std"
export RUST_BACKTRACE=1
kani verify-std -Z unstable-options "${TMP_DIR}/library" --target-dir "${TMP_DIR}/target"

# Cleanup
rm -r ${TMP_DIR}
