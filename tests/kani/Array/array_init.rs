// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn init_int() {
    let a = [4u8; 6];
    let i: usize = kani::any();
    kani::assume(i < 6);
    assert_eq!(a[i], 4);
}

#[kani::proof]
fn init_option() {
    let a = [Some(4u8); 6];
    let i: usize = kani::any();
    kani::assume(i < 6);
    assert_eq!(a[i], Some(4));
}
