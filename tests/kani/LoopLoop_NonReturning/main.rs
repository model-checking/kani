// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(10)]
fn main() {
    let mut a: u32 = kani::any();

    if a < 1024 {
        loop {
            a = a / 2;

            if a == 0 {
                break;
            }
        }

        assert!(a == 0);
    }
}
