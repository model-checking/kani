// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Fails with a type safety error - see TODO in rvalue.rs::codegen_misc_cast()
fn main() {
    let s = String::from("hello");
    let _b = s.into_boxed_str();
}
