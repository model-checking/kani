// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests the handling of IntoIter associated types for Iterators.
//! For Iterators, the IntoIter associated type is the type of the iterator itself.
//! In this case, Kani should treat the types as the same type.

use std::str::Chars;

struct MyStruct<'a> {
    copy1: Option<<Chars<'a> as IntoIterator>::IntoIter>,
    copy2: Option<Chars<'a>>,
}

impl<'a> MyStruct<'a> {
    fn new(source: Chars<'a>) -> Self {
        MyStruct { copy1: Some(source.clone()), copy2: Some(source) }
    }
}

#[kani::proof]
#[kani::unwind(3)]
pub fn check_into_iter_type() {
    let original = "h";
    let mut wrapper = MyStruct::new(original.chars());
    assert!(wrapper.copy1.unwrap().eq(wrapper.copy2.unwrap()));
}
