// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Push 5 elements to force the vector to resize, then check that the values were correctly copied.
#[kani::proof]
fn main() {
    let mut v = Vec::new();
    v.push(72);
    v.push(2);
    v.push(3);
    v.push(4);
    v.push(5);
    assert!(v[0] == 72);
    assert!(v[1] == 2);
    assert!(v[2] == 3);
    assert!(v[3] == 4);
    assert!(v[4] == 5);
}
