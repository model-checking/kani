// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(5)]
#[kani::solver(minisat)]
fn configured_harness() {
    let n: u8 = kani::any();
    kani::assume(n <= 3);
    let mut i: u8 = 0;
    let mut acc: u32 = 0;
    while i < n {
        acc += 1;
        i += 1;
    }
    assert!(acc == n as u32);
}

#[kani::proof]
fn coverage_harness() {
    let x: u8 = kani::any();
    if x < 10 {
        assert!(x < 10);
    } else {
        assert!(x >= 10);
    }
}
