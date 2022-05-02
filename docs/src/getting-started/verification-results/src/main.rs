// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
// ANCHOR: success_example
fn success_example() {
    let mut sum = 0;
    for i in 1..4 {
        sum += i;
    }
    assert_eq!(sum, 6);
}
// ANCHOR_END: success_example

#[kani::proof]
// ANCHOR: failure_example
fn failure_example() {
    let arr = [1, 2, 3];
    assert_ne!(arr.len(), 3);
}
// ANCHOR_END: failure_example

#[kani::proof]
// ANCHOR: unreachable_example
fn unreachable_example() {
    let x = 5;
    let y = x + 2;
    if x > y {
        assert!(x < 8);
    }
}
// ANCHOR_END: unreachable_example

#[kani::proof]
// ANCHOR: undetermined_example
fn undetermined_example() {
    let mut x = 0;
    unsupp(&mut x);
    assert!(x == 0);
}

#[feature(asm)]
fn unsupp(x: &mut u8) {
    unsafe {
        std::arch::asm!("nop");
    }
}

// ANCHOR_END: undetermined_example

fn main() {}
