// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

include!("../../rmc-prelude.rs");

fn main() {
    let i: i32 = __nondet();
    __VERIFIER_assume(i < 10);
    __VERIFIER_expect_fail(i > 20, "Blocked by assumption above.");
}
