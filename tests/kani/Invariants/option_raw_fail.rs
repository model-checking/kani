// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that kani::any_raw will generate unsconstrainted values for Option<T,E>.

extern crate kani;

use kani::Invariant;

struct MyType {
    pub val: char,
}

unsafe impl kani::Invariant for MyType {
    fn is_valid(&self) -> bool {
        self.val.is_valid()
    }
}

fn main() {
    let option: Option<MyType> = unsafe { kani::any_raw() };
    if let Some(ref v) = option {
        kani::expect_fail(v.is_valid(), "No guarantee about the underlying value");
    }
}
