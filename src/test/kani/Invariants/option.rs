// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the Invariant implementation for Option respect underlying types invariant.

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
    let option: Option<MyType> = rmc::any();
    match option {
        Some(v) => assert!(v.is_valid() && v.val <= char::MAX),
        None => (),
    }
}
