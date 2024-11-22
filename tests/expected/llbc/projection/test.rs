// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles simple projection

struct MyStruct {
    a: i32,
    b: i32,
}

enum MyEnum0 {
    A(MyStruct, i32),
    B,
}

enum MyEnum {
    A(MyStruct, MyEnum0),
    B((i32, i32)),
}

fn enum_match(e: MyEnum) -> i32 {
    match e {
        MyEnum::A(s, e0) => match e0 {
            MyEnum0::A(s1, b) => s1.a + b,
            MyEnum0::B => s.a + s.b,
        },
        MyEnum::B((a, b)) => a + b,
    }
}

#[kani::proof]
fn main() {
    let s = MyStruct { a: 1, b: 2 };
    let s0 = MyStruct { a: 1, b: 2 };
    let e = MyEnum::A(s, MyEnum0::A(s0, 1));
    let i = enum_match(e);
}
