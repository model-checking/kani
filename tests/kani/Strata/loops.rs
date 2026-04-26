// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test loop support in Strata backend

#[kani::proof]
fn test_simple_loop() {
    let mut x: u32 = 0;
    let mut count: u32 = 0;

    while count < 5 {
        x = x + 1;
        count = count + 1;
    }

    assert!(x == 5);
}

#[kani::proof]
fn test_for_loop() {
    let mut sum: u32 = 0;

    for i in 0..3 {
        sum = sum + i;
    }

    assert!(sum == 3); // 0 + 1 + 2
}

#[kani::proof]
fn test_loop_with_break() {
    let mut x: u32 = 0;

    loop {
        x = x + 1;
        if x >= 10 {
            break;
        }
    }

    assert!(x == 10);
}
