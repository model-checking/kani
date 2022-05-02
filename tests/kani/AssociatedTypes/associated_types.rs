// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests the behavior of associated types.
//! See https://doc.rust-lang.org/book/ch19-03-advanced-traits.html for more details.

/// Covert Source -> Target
trait Convert {
    type Source;
    type Target;

    fn convert(&self) -> Self::Target;
}

/// Dummy trait that returns an u8
trait U8Wrapper {
    fn get(&self) -> u8;
}

impl<T: U8Wrapper> Convert for T {
    type Source = T;
    type Target = u8;

    fn convert(&self) -> u8 {
        self.get()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct MyU8 {
    inner: u8,
}

impl U8Wrapper for MyU8 {
    fn get(&self) -> u8 {
        self.inner
    }
}

#[kani::proof]
pub fn check_source_type() {
    let obj1 = Some(MyU8 { inner: 10 });
    let obj2: Option<<MyU8 as Convert>::Source> = obj1;
    assert_eq!(obj1.unwrap(), obj2.unwrap());
}

#[kani::proof]
pub fn check_fn_convert() {
    let val: u8 = kani::any();
    let obj1 = Some(MyU8 { inner: val });
    assert_eq!(obj1.unwrap().get(), val);
    assert_eq!(obj1.unwrap().convert(), val);
}

#[kani::proof]
pub fn check_dyn_convert() {
    let val: u8 = kani::any();
    let obj1 = MyU8 { inner: val };
    let con: &dyn Convert<Source = MyU8, Target = u8> = &obj1;
    assert_eq!(con.convert(), val);
}
