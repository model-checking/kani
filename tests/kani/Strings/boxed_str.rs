// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let s = String::from("hello");
    let _b = s.into_boxed_str();
}
