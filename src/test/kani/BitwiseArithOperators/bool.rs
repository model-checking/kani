// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let a: bool = rmc::any();
    let b: bool = rmc::any();
    let c = a ^ b;
    assert!((a == b && !c) || (a != b && c));
}
