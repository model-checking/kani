// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn __VERIFIER_assume(cond: bool) {
    unimplemented!()
}

fn __nondet<T>() -> T {
    unimplemented!()
}

fn main() {
    let i: i32 = __nondet();
    __VERIFIER_assume(i < 10);
    assert!(i > 20);
}
