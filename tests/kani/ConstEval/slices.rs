// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn point_slice() {
    let point: &[(u8, u32)] = &[(0, u32::MAX), (u8::MAX, 0)];
    assert_eq!(point.len(), 2);
    assert_eq!(point[0].0, 0);
    assert_eq!(point[0].1, u32::MAX);
}

#[kani::proof]
fn points() {
    let point: &[(u8, u8)] = &[(0, u8::MAX), (10, 231)];
    assert_eq!(point.len(), 2);
    assert_eq!(point[0].0, 0);
    assert_eq!(point[0].1, u8::MAX);
}
