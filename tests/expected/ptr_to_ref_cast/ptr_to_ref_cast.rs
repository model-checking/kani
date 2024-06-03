// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test case checks that raw pointer validity is checked before converting it to a reference, e.g., &(*ptr).

// 1. Original example.

struct Store<'a, const LEN: usize> {
    data: [&'a i128; LEN],
}

impl<'a, const LEN: usize> Store<'a, LEN> {
    pub fn from(var: &i64) -> Self {
        let ref1: *const i64 = var;
        let ref2: *const i128 = ref1 as *const i128;
        unsafe {
            Store { data: [&*ref2; LEN] } // ---- THIS LINE SHOULD FAIL
        }
    }
}

#[kani::proof]
pub fn check_store() {
    let val = 1;
    let broken = Store::<3>::from(&val);
    assert_eq!(broken.data.len(), 3)
}

// 2. Make sure the error is raised when casting to a simple type of a larger size.

pub fn larger_deref(var: &i64) {
    let ref1: *const i64 = var;
    let ref2: *const i128 = ref1 as *const i128;
    let ref3: &i128 = unsafe { &*ref2 }; // ---- THIS LINE SHOULD FAIL
}

#[kani::proof]
pub fn check_larger_deref() {
    let var: i64 = kani::any();
    larger_deref(&var);
}

// 3. Make sure the error is raised when casting to a simple type of a larger size and storing the result in a pointer.

pub fn larger_deref_into_ptr(var: &i64) {
    let ref1: *const i64 = var;
    let ref2: *const i128 = ref1 as *const i128;
    let ref3: *const i128 = unsafe { &*ref2 }; // ---- THIS LINE SHOULD FAIL
}

#[kani::proof]
pub fn check_larger_deref_into_ptr() {
    let var: i64 = kani::any();
    larger_deref_into_ptr(&var);
}

// 4. Make sure the error is raised when casting to a struct of a larger size.

#[derive(kani::Arbitrary)]
struct Foo {
    a: u8,
}

#[derive(kani::Arbitrary)]
struct Bar {
    a: u8,
    b: u64,
    c: u64,
}

pub fn larger_deref_struct(var: &Foo) {
    let ref1: *const Foo = var;
    let ref2: *const Bar = ref1 as *const Bar;
    let ref3: &Bar = unsafe { &*ref2 }; // ---- THIS LINE SHOULD FAIL
}

#[kani::proof]
pub fn check_larger_deref_struct() {
    let var: Foo = kani::any();
    larger_deref_struct(&var);
}

// 5. Make sure the error is not raised if the target size is smaller.

pub fn smaller_deref(var: &i64, var_struct: &Bar) {
    let ref1: *const i64 = var;
    let ref2: *const i32 = ref1 as *const i32;
    let ref3: &i32 = unsafe { &*ref2 };

    let ref1_struct: *const Bar = var_struct;
    let ref2_struct: *const Foo = ref1_struct as *const Foo;
    let ref3_struct: &Foo = unsafe { &*ref2_struct };
}

#[kani::proof]
pub fn check_smaller_deref() {
    let var: i64 = kani::any();
    let var_struct: Bar = kani::any();
    smaller_deref(&var, &var_struct);
}

// 6. Make sure the error is not raised if the target size is the same.

pub fn equal_size_deref(var: &i64, var_struct: &Foo) {
    let ref1: *const i64 = var;
    let ref2: &i64 = unsafe { &*ref1 };

    let ref1_struct: *const Foo = var_struct;
    let ref2_struct: &Foo = unsafe { &*ref1_struct };
}

#[kani::proof]
pub fn check_equal_size_deref() {
    let var: i64 = kani::any();
    let var_struct: Foo = kani::any();
    equal_size_deref(&var, &var_struct);
}

// 7. Make sure the check works with ZSTs.

#[derive(kani::Arbitrary)]
struct Zero;

pub fn zst_deref(var_struct: &Foo, var_zst: &Zero) {
    let ref1_struct: *const Foo = var_struct;
    let ref2_struct: *const Zero = ref1_struct as *const Zero;
    let ref3_struct: &Zero = unsafe { &*ref2_struct };

    let ref1_zst: *const Zero = var_zst;
    let ref2_zst: *const Foo = ref1_zst as *const Foo;
    let ref3_zst: &Foo = unsafe { &*ref2_zst }; // ---- THIS LINE SHOULD FAIL
}

#[kani::proof]
pub fn check_zst_deref() {
    let var_struct: Foo = kani::any();
    let var_zst: Zero = kani::any();
    zst_deref(&var_struct, &var_zst);
}
