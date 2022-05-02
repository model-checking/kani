// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the Invariant implementation for Result respect underlying types invariant.

extern crate kani;

use kani::Invariant;

#[derive(PartialEq)]
enum Error {
    Error1,
    Error2,
}

struct MyType {
    pub val: i32,
    pub is_negative: bool,
}

unsafe impl kani::Invariant for MyType {
    fn is_valid(&self) -> bool {
        (self.is_negative && self.val < 0) || (!self.is_negative && self.val >= 0)
    }
}

unsafe impl kani::Invariant for Error {
    fn is_valid(&self) -> bool {
        *self == Error::Error1 || *self == Error::Error2
    }
}

#[kani::proof]
fn main() {
    let result: Result<MyType, Error> = kani::any();
    match result {
        Ok(v) => assert!(v.is_valid()),
        Err(e) => assert!(e.is_valid()),
    }
}
