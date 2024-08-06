// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that Kani reports an unreachable cover property as such

#[kani::proof]
fn cover_unreachable() {
    let x: i32 = kani::any();
    if x > 10 {
        if x < 5 {
            kani::cover_or_fail!(x == 2); // unreachable
        }
    } else {
        if x > 20 {
            kani::cover_or_fail!(x == 30, "Unreachable with a message"); // unreachable
        } else {
            kani::cover_or_fail!(x == 5); // satisfiable
        }
    }
}
