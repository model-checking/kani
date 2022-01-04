// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// rmc-verify-fail
// Ensure that rmc::any_raw will generate unsconstrainted values for Option<T,E>.

extern crate rmc;

use rmc::Invariant;

struct MyType {
    pub val: char,
}

unsafe impl rmc::Invariant for MyType {
    fn is_valid(&self) -> bool {
        self.val.is_valid()
    }
}

fn main() {
    let option: Option<MyType> = unsafe { rmc::any_raw() };
    if let Some(ref v) = option {
        rmc::expect_fail(v.is_valid(), "No guarantee about the underlying value");
    }
}
