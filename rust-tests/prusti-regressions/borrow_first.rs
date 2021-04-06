// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn some_condition(r: &i32) -> bool {
    *r > 0
}

fn foo(vec: &mut Vec<i32>) -> &i32 {
    if some_condition(&vec[0]) {
        return &vec[0];
    }

    vec.push(5);
    let last = vec.len() - 1;
    &vec[last]
}

fn main() {
    let mut v = vec![-1, 2, 3];
    let r = foo(&mut v);
    assert!(*r > 0);
    assert!(*r == 5);
}
