// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn foo() {
    let x: i32 = kani::any();
    if x > 5 {
        // fails
        assert!(x < 4);
        if x < 3 {
            // unreachable
            assert!(x == 2);
        }
    } else {
        // passes
        assert!(x <= 5);
    }
}
