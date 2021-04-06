// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![no_std]
#![crate_type = "lib"]

extern crate foo;

pub fn bar() -> foo::A { foo::foo() }
