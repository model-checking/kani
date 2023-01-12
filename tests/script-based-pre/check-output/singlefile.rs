// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub struct PrettyStruct;

#[kani::proof]
pub fn main() {
    pretty_function(PrettyStruct);
    monomorphize::<()>();
    monomorphize::<usize>();
    let x = [true; 2];
    let ref_to_str = &"";
    let test_enum = TestEnum::Variant1(true);
}

pub fn pretty_function(argument: PrettyStruct) -> PrettyStruct {
    argument
}

pub fn monomorphize<T>() {}

enum TestEnum {
    Variant1(bool),
    Variant2,
}
