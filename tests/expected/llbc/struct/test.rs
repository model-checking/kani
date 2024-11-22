// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles simple struct

struct MyStruct {
    a: i32,
    b: bool,
}

fn struct_project(s: MyStruct) -> i32 {
    s.a
}

#[kani::proof]
fn main() {
    let s = MyStruct { a: 1, b: true };
    let a = struct_project(s);
}
