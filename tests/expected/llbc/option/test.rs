// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles trait clauses in generic args

fn both_none<T,U>(a: Option<T>, b: Option<U>)-> bool{
    match a {
        None => match b {
            None => true,
            _ => false
        },
        _ => false
    }
}

#[kani::proof]
fn main() {
    let a = Some(1 as u32);
    let b = Some(2);
    let i = both_none(a, b);
}
