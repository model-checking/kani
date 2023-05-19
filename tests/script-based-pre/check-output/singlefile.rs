// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Add a dummy data so PrettyStruct doesn't get removed from arg list.
pub struct PrettyStruct(u32);

#[kani::proof]
pub fn main() {
    pretty_function(PrettyStruct(5));
    monomorphize::<()>();
    monomorphize::<usize>();
    let x = [true; 2];
    let ref_to_str = &"";
    assert!(ref_to_str.is_empty());
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
