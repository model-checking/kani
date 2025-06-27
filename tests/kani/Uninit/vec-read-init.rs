// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

#[kani::proof]
fn vec_read_init() {
    let mut v: Vec<u8> = Vec::with_capacity(10);
    unsafe { *v.as_mut_ptr().add(5) = 0x42 };
    let def = unsafe { *v.as_ptr().add(5) }; // Accessing previously initialized byte is valid.
    let x = def + 1;
}
