// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles simple projection

struct MyStruct {
    a: i32,
    b: i32,
}

enum MyEnum {
    A(MyStruct, i32),
    B,
}

fn enum_match(e: MyEnum) -> i32 {
    match e {
        MyEnum::A(s, i) => s.a + i,
        MyEnum::B => 0,
    }
}

#[kani::proof]
fn main() {
    let s = MyStruct { a: 1, b: 2 };
    let e = MyEnum::A(s, 1);
    let i = enum_match(e);
}
