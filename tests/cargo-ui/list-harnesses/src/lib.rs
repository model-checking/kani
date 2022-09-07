// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The --list-harnesses flag is to return a consumable JSON.
//! Can't test for exact output but this flag should not return anything but JSON

fn estimate_size(x: u32) -> u32 {
    if x < 256 {
        if x < 128 {
            return 1;
        } else {
            return 3;
        }
    } else if x < 1024 {
        if x > 1022 {
            panic!("Oh no, a failing corner case!");
        } else {
            return 5;
        }
    } else {
        if x < 2048 {
            return 7;
        } else {
            return 9;
        }
    }
}

// ANCHOR: kani
#[cfg(kani)]
#[kani::proof]
fn check_estimate_size() {
    let x: u32 = kani::any();
    estimate_size(x);
}

#[kani::proof]
fn check_estimate_size_2() {
    let y: u32 = kani::any();
    estimate_size(y);
}
