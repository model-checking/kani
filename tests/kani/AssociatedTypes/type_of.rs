// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests the behavior of TypeId for structs with associated types in their member declaration.
//!
//! See https://github.com/model-checking/kani/issues/1124 for more details.
use std::any::TypeId;

trait Associated {
    type Typ;
}

impl<T> Associated for T {
    type Typ = T;
}

struct Wrapper<T>(T);

struct MyStruct {
    first: Wrapper<u8>,
    second: Wrapper<<u8 as Associated>::Typ>,
}

fn same_type<T: 'static, U: 'static>(_: T, _: U) -> bool {
    TypeId::of::<T>() == TypeId::of::<U>()
}

#[kani::proof]
fn check_type() {
    let mine = MyStruct { first: Wrapper(10), second: Wrapper(10) };
    assert!(same_type(mine.first, mine.second));
}
