// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles simple generic args for option

fn add_opt(x: Option<i32>, y: Option<i32>) -> Option<i32> {
    match x {
        Some(u) => match y {
            Some(v) => Some(u + v),
            _ => None,
        },
        _ => None,
    }
}

#[kani::proof]
fn main() {
    let e = add_opt(Some(1), Some(2));
}
