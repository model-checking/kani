// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that none of these operations trigger spurious overflow checks.

macro_rules! verify_no_overflow {
    ($cf: ident, $uf: tt) => {{
        let a: u8 = kani::nondet();
        let b: u8 = kani::nondet();
        let checked = a.$cf(b);
        kani::assume(checked.is_some());
        let unchecked = unsafe { a $uf b };
        assert!(checked.unwrap() == unchecked);
    }};
}

fn main() {
    verify_no_overflow!(checked_add, +);
    verify_no_overflow!(checked_sub, -);
    verify_no_overflow!(checked_mul, *);
}
