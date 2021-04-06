// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::collections::VecDeque;

fn __nondet<T>() -> T {
    unimplemented!()
}

fn main() {
    let x = __nondet();
    let y = __nondet();
    let mut q: VecDeque<i32> = VecDeque::new();
    q.push_back(x);
    q.push_back(y);
    assert!(q.len() == 2);
    assert!(q.pop_front().unwrap() == x);
    assert!(q.pop_front().unwrap() == y);
}
