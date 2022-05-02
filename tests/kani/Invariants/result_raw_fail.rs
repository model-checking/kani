// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that kani::any_raw will generate unsconstrainted values for Result<T,E>.

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
        matches!(*self, Error::Error1 | Error::Error2)
    }
}

#[kani::proof]
fn main() {
    let result: Result<MyType, Error> = unsafe { kani::any_raw() };
    if let Ok(ref v) = result {
        kani::expect_fail(v.is_valid(), "No guarantee about the underlying value");
    }
    if let Err(e) = result {
        kani::expect_fail(e.is_valid(), "No guarantee about the underlying error");
    } else {
        kani::expect_fail(false, "This is also reachable since the enum is unconstrained.");
    }
}
