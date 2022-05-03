// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    let mut x = 1;
    add_two(&mut x);
    assert!(x == 3);
}

fn add_two(x: *mut u32) {
    unsafe {
        *x += 2;
    }
}
