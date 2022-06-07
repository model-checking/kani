// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn a_check() {
    let v = vec![1, 2, 3];
    assert_eq!(v.len(), 3);
}
