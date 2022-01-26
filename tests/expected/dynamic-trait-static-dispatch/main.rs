// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::io::{self};

pub trait Foo {
    fn a(&self) -> u32;
    fn b(&self) -> u32;
}

pub struct Bar {}

impl Foo for Bar {
    fn a(&self) -> u32 {
        return 3;
    }

    fn b(&self) -> u32 {
        return 5;
    }
}

// this example works with static dispatch, so should work also while dynamic dispatch is not yet resolved
fn main() {
    let bar = Bar {};
    assert!(bar.a() == 3);
    assert!(bar.b() == 5);
}
