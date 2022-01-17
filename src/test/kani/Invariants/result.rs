// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the Invariant implementation for Result respect underlying types invariant.

extern crate rmc;

use rmc::Invariant;

#[derive(PartialEq)]
enum Error {
    Error1,
    Error2,
}

struct MyType {
    pub val: i32,
    pub is_negative: bool,
}

unsafe impl rmc::Invariant for MyType {
    fn is_valid(&self) -> bool {
        (self.is_negative && self.val < 0) || (!self.is_negative && self.val >= 0)
    }
}

unsafe impl rmc::Invariant for Error {
    fn is_valid(&self) -> bool {
        matches!(*self, Error::Error1 | Error::Error2)
    }
}

fn main() {
    let result: Result<MyType, Error> = rmc::any();
    match result {
        Ok(v) => assert!(v.is_valid()),
        Err(e) => assert!(e.is_valid()),
    }
}
