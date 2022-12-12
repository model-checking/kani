// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that the unwinding assertions pass for nested loops for
//! which there's a backedge into the middle of the loop

#[kani::proof]
#[kani::unwind(3)]
fn check_unwind_assertion() {
    let a: &[i32] = &[0, 0];
    for &e in a {
        assert_eq!(e, 0);
        for i in e..1 {
            assert_eq!(i, 0);
        }
    }
}
