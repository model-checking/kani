// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: autoharness -Z autoharness

#[allow(unused)]
fn takes_ref(x: &i32) -> i32 {
    x + x
}

#[allow(unused)]
fn takes_refs(x: &i32, y: &i32) -> i32 {
    x - y
}

#[allow(unused)]
fn takes_refs_and_other(x: &i32, y: &i32, z: i32) -> i32 {
    x * y % z
}
