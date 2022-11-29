// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect error
// kani-verify-fail

fn double(a: u32) -> u32 {
    a * 2
}

#[kani::proof]
pub fn main() {
    let a = kani::any();
    if a <= std::u32::MAX / 2 {
        // avoid overflow
        let b = double(a);
        assert!(b != 2 * a);
    }
}
