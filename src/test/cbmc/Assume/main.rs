// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

include!("../../rmc-prelude.rs");

fn main() {
    let i: i32 = __nondet();
    __VERIFIER_assume(i < 10);
    assert!(i < 20);
}
