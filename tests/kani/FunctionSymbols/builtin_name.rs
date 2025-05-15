// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Function with the same name as a known built-in, but we provide an alternative implementation
// instead of using the built-in.
#[no_mangle]
fn copysign(a: f64, _b: f64) -> f64 {
    a
}

#[kani::proof]
pub fn harness() {
    let a: f64 = kani::any();
    let b: f64 = kani::any();
    copysign(a, b);
}
