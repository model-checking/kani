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
kani_core::kani_lib_core!();

#[cfg(kani)]
#[unstable(feature = "kani", issue = "none")]
pub mod verify {
    use crate::kani;

    #[kani::proof]
    pub fn harness() {
        kani::assert(true, "yay");
    }

    #[kani::proof_for_contract(fake_function)]
    fn dummy_proof() {
        fake_function(true);
    }

    /// Add a `rustc_diagnostic_item` to ensure this works.
    /// See <https://github.com/model-checking/kani/issues/3251> for more details.
    #[kani::requires(x == true)]
    #[rustc_diagnostic_item = "fake_function"]
    fn fake_function(x: bool) -> bool {
        x
    }

    #[kani::proof_for_contract(dummy_read)]
    fn check_dummy_read() {
        let val: char = kani::any();
        assert_eq!(unsafe { dummy_read(&val) }, val);
    }

    /// Ensure we can verify constant functions.
    #[kani::requires(kani::mem::can_dereference(ptr))]
    #[rustc_diagnostic_item = "dummy_read"]
    const unsafe fn dummy_read<T: Copy>(ptr: *const T) -> T {
        *ptr
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
kani verify-std -Z unstable-options "${TMP_DIR}/library" --target-dir "${TMP_DIR}/target" -Z function-contracts -Z stubbing

# Cleanup
rm -r ${TMP_DIR}
