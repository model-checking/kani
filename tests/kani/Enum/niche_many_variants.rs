// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Testcase for niche encoding where there are multiple variants but only one contains
//! non-zero sized data with niche, making it a great candidate for niche optimization.
#[derive(PartialEq)]
enum MyEnum {
    NoFields,
    DataFul(bool),
    UnitFields((), ()),
    ZSTField(ZeroSized),
    ZSTStruct { field: ZeroSized, unit: () },
}

#[derive(PartialEq)]
struct ZeroSized {}

impl ZeroSized {
    fn works(&self) -> bool {
        true
    }
}

impl MyEnum {
    fn create_no_field() -> MyEnum {
        MyEnum::NoFields
    }

    fn create_data_ful(data: bool) -> MyEnum {
        MyEnum::DataFul(data)
    }

    fn create_unit() -> MyEnum {
        MyEnum::UnitFields((), ())
    }

    fn create_zst_field() -> MyEnum {
        MyEnum::ZSTField(ZeroSized {})
    }

    fn create_zst_struct() -> MyEnum {
        MyEnum::ZSTStruct { field: ZeroSized {}, unit: () }
    }
}

#[kani::proof]
fn check_is_niche() {
    // Ensure we are testing a case of niche optimization.
    assert_eq!(std::mem::size_of::<MyEnum>(), 1);
    assert_eq!(std::mem::size_of::<bool>(), 1);
}

#[kani::proof]
fn check_niche_no_fields() {
    // Check the behavior for the dataful variant.
    let x = MyEnum::create_no_field();
    assert!(matches!(x, MyEnum::NoFields));
}

#[kani::proof]
fn check_niche_data_ful() {
    // Check the behavior for the dataful variant.
    let x = MyEnum::create_data_ful(true);
    assert!(matches!(x, MyEnum::DataFul(true)));
}

#[kani::proof]
fn check_niche_unit_fields() {
    // Check the behavior for the variant with one unit field.
    let x = MyEnum::create_unit();
    assert_eq!(x, MyEnum::UnitFields((), ()));
    if let MyEnum::UnitFields(ref v, ..) = &x {
        assert_eq!(std::mem::size_of_val(v), 0);
    }
}

#[kani::proof]
fn check_niche_zst_field() {
    // Check the behavior for the variant with one unit field.
    let x = MyEnum::create_zst_field();
    assert_eq!(x, MyEnum::ZSTField(ZeroSized {}));
    if let MyEnum::ZSTField(ref field) = &x {
        assert!(field.works());
    }
}

#[kani::proof]
fn check_niche_zst_struct() {
    // Check the behavior for the variant with one unit field.
    let x = MyEnum::create_zst_struct();
    assert!(matches!(x, MyEnum::ZSTStruct { .. }));
    if let MyEnum::ZSTStruct { ref field, ref unit } = &x {
        assert_eq!(std::mem::size_of_val(unit), 0);
        assert!(field.works());
    }
}
