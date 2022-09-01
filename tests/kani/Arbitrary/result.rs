// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the Arbitrary implementation for Result respect underlying types invariant.

extern crate kani;

#[derive(PartialEq)]
enum Error {
    Error1,
    Error2,
}

struct MyType {
    pub val: i32,
    pub is_negative: bool,
}

impl kani::Arbitrary for MyType {
    fn any() -> Self {
        let val: i32 = kani::any();
        let is_negative = val < 0;
        Self { val, is_negative }
    }
}

impl kani::Arbitrary for Error {
    fn any() -> Self {
        if kani::any() { Error::Error1 } else { Error::Error2 }
    }
}

#[kani::proof]
fn main() {
    let result: Result<MyType, Error> = kani::any();
    match result {
        Ok(v) => assert!(v.is_negative || v.val >= 0),
        Err(Error::Error1) => assert!(result.is_err()),
        Err(Error::Error2) => assert!(result.is_err()),
    }
}
