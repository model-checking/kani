// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that the Invariant implementation for Option respect underlying types invariant.

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
    let option: Option<MyType> = kani::any();
    match option {
        Some(v) => assert!(v.is_valid() && v.val <= char::MAX),
        None => (),
    }
}
