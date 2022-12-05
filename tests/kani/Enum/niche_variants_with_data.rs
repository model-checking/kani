// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This test checks the layout of the niche optimization with multiple data.

enum MyEnum {
    Flag1(Option<bool>),
    Flag2(u8, Option<bool>),
}

trait IsTrue {
    fn check(&self) -> bool;
}

impl IsTrue for MyEnum {
    fn check(&self) -> bool {
        match self {
            MyEnum::Flag1(Some(val)) | MyEnum::Flag2(_, Some(val)) => *val,
            _ => false,
        }
    }
}

#[kani::proof]
pub fn check_size() {
    let flag = MyEnum::Flag1(Some(true));
    assert_eq!(std::mem::size_of_val(&flag), 2);
}

#[kani::proof]
pub fn check_val() {
    let flag = MyEnum::Flag2(0, None);
    let is_true: &dyn IsTrue = &flag;
    assert_eq!(is_true.check(), false);
}
