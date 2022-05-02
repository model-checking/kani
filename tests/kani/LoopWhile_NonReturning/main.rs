// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(11)]
fn main() {
    let mut a: u32 = kani::any();

    if a < 1024 {
        while a > 0 {
            a = a / 2;
        }

        assert!(a == 0);
    }
}
