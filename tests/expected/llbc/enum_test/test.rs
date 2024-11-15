// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles simple enum


enum MyEnum {
    A(i32),
    B,
}


fn enum_match(e: MyEnum) -> i32 {
    match e {
        MyEnum::A(i) => i ,
        MyEnum::B => 0 ,
    }
}



#[kani::proof]
fn main() {
    let e = MyEnum::A(1);
    let i = enum_match(e);
}
