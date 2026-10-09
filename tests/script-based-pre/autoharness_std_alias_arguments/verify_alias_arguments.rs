// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Code injected into `core`: functions that are only correct if their arguments do not alias.
#[cfg(kani)]
kani_core::kani_lib!(core);

#[cfg(kani)]
#[unstable(feature = "kani", issue = "none")]
pub mod verify_alias_arguments {
    /// Fails under `--alias-arguments` only: `b` may then be the same reference as `a`.
    pub fn refs_distinct(a: &u8, b: &u8) {
        assert!(!crate::ptr::eq(a, b));
    }

    /// Fails under `--alias-arguments` only: `q` may then be the same pointer as `p`. Null is
    /// excluded because both pointers may be null in either mode.
    pub fn ptrs_distinct(p: *const u8, q: *const u8) {
        if !p.is_null() && !q.is_null() {
            assert!(p != q);
        }
    }
}
