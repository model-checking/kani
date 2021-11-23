// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(never_type)]
use std::convert::Infallible;

/// Test using the never type
pub fn foo(never: !) -> i32 {
    return 1;
}

pub fn bar(infalliable: Infallible) -> i32 {
    return 1;
}

// Give an empty main to make rustc happy.
fn main() {}
