// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub fn main() {
    let x: u8 = rmc::nondet();
    let y = x;
    assert!(x == y);
}
