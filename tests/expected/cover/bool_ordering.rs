// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn bool_order_covers() {
    let a: bool = kani::any();
    let b: bool = kani::any();

    kani::cover!(a < b, "a can be less than b");
    kani::cover!(a < b && b < a, "a and b cannot both be less than each other");
}
