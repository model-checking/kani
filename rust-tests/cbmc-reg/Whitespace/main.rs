// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut iter = "A few words".split_whitespace();
    match iter.next() {
        None => assert!(false),
        Some(x) => assert!(x == "A"),
    }
}
