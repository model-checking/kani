// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Regression test for <https://github.com/model-checking/kani/issues/4829>.
//!
//! Any allocation reaches `NonNull<[u8]>`, whose field is `pattern_type!(*const [u8] is !null)`:
//! a pattern over a wide pointer, so its ABI is `ScalarPair`, not `Scalar`. The validity pass
//! used to assert that a pattern type is always a scalar and crash the compiler. It must instead
//! check the data pointer's `!null` range, so that allocating code verifies and a null data
//! pointer is still reported as an invalid value.

use std::ptr::NonNull;

#[kani::proof]
fn takes_box() {
    let b = Box::new(kani::any::<u32>());
    assert_eq!(*b, *b);
}

/// A null data pointer must still be reported: the check is enforced, not skipped. The value is
/// used so that the transmute is not optimized away before the pass sees it.
#[kani::proof]
fn null_data_pointer() {
    let nn = unsafe { std::mem::transmute::<(usize, usize), NonNull<[u8]>>((0, 3)) };
    assert_eq!(nn.len(), 3);
}
