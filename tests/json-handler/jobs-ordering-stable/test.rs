// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod extra;

fn work(n: u32) -> u32 {
    let mut acc: u32 = 0;
    let mut i: u32 = 0;
    while i < n {
        acc = acc.wrapping_add(i);
        i += 1;
    }
    acc
}

#[kani::proof]
#[kani::unwind(33)]
fn z_heavy_in_test_rs() {
    let n: u32 = kani::any();
    kani::assume(n <= 32);
    let _ = work(n);
}

#[kani::proof]
fn a_light_in_test_rs() {
    let x: u32 = kani::any();
    kani::assume(x < 4);
    assert!(x < 4);
}
