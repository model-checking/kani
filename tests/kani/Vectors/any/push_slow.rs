// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Variant of tests/kani/Vector/push.rs using any_vec. Slow due to
//! performance issues involving any_vec. See #1329
#[kani::proof]
fn main() {
    let mut v: Vec<isize> = kani::vec::any_vec::<_, 0>();
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
