// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![no_std]
#![crate_type = "lib"]

#[allow(dead_code)]
pub struct A {
    x: i32,
}

pub fn foo() -> A {
    A {
        x: 10,
    }
}