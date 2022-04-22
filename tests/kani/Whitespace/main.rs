// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
#[kani::unwind(2)]
fn main() {
    let mut iter = "A few words".split_whitespace();
    match iter.next() {
        None => assert!(false),
        Some(x) => assert!(x == "A"),
    }
}
