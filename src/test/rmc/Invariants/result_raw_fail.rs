// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// rmc-verify-fail
// Ensure that rmc::any_raw will generate unsconstrainted values for Result<T,E>.

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
    let result: Result<MyType, Error> = unsafe { rmc::any_raw() };
    if let Ok(ref v) = result {
        rmc::expect_fail(v.is_valid(), "No guarantee about the underlying value");
    }
    if let Err(e) = result {
        rmc::expect_fail(e.is_valid(), "No guarantee about the underlying error");
    } else {
        rmc::expect_fail(false, "This is also reachable since the enum is unconstrained.");
    }
}
