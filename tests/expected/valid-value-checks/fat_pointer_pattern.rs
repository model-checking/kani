// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Regression test for <https://github.com/model-checking/kani/issues/4829>.
//!
//! Any allocation reaches `NonNull<[u8]>`, whose field is `pattern_type!(*const [u8] is !null)`:
//! a pattern over a wide pointer, so its ABI is `ScalarPair`, not `Scalar`. The validity pass
//! used to assert that a pattern type is always a scalar and crash the compiler. It must instead
//! report the check as unsupported, like it does for `char` patterns.

#[kani::proof]
fn takes_box() {
    let b = Box::new(kani::any::<u32>());
    assert_eq!(*b, *b);
}
