// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

fn main() {
    let a: bool = __nondet();
    let b: bool = __nondet();
    let c = a ^ b;
    assert!((a == b && !c) || (a != b && c));
}
