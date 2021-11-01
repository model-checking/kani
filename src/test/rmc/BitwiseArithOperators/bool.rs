// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub fn main() {
    let a: bool = rmc::nondet();
    let b: bool = rmc::nondet();
    let c = a ^ b;
    assert!((a == b && !c) || (a != b && c));
}
