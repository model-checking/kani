// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that the remainder operator works with floating point values (see issue #2669)

#[kani::proof]
fn rem_float() {
    let dividend = 0.5 * f32::from(kani::any::<i8>());
    let divisor = 0.5 * f32::from(kani::any::<i8>());
    kani::assume(divisor != 0.0);
    let result = dividend % divisor;
    assert!(result == 0.0 || result.is_normal());
}
