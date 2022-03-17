// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// cbmc-flags: --unwind 6

#[kani::proof]
fn main() {
    let name: &str = "hello";
    assert!(name == "hello");
}
