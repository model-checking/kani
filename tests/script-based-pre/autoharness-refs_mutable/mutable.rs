// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: autoharness -Z autoharness

#[allow(unused)]
fn takes_mut_ref(x: &mut i32) -> i32 {
    *x = *x + *x;
    *x
}

#[allow(unused)]
fn takes_mut_refs(x: &mut i32, y: &mut i32) -> i32 {
    *x = *x - *y;
    *y = *x - *y;
    *x
}

#[allow(unused)]
fn takes_mut_refs_and_other(x: &i32, y: &mut i32, z: i32) -> i32 {
    *y = x * *y % z;
    *y
}
