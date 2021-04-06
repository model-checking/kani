// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn foo(x: &[i32; 5]) -> &[i32] {
    &x[..]
}

fn bar(x: &[i32; 5]) -> &[i32] {
    &x[1..4]
}

fn main() {
    let x = [1, 2, 3, 4, 5];
    let y = foo(&x);
    let z = bar(&x);
    assert!(y.len() == 5);
    assert!(y[1] == 2);
    assert!(z.len() == 3);
}
