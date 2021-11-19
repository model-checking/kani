// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let a: bool = unsafe { rmc::nondet() };
    let b: bool = unsafe { rmc::nondet() };
    let c = a ^ b;
    assert!((a == b && !c) || (a != b && c));
}
