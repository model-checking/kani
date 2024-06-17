// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

#[kani::proof]
fn main() {
    let mut v: Vec<u8> = Vec::with_capacity(10);
    unsafe { *v.as_mut_ptr().add(4) = 0x42 };
    let def = unsafe { *v.as_ptr().add(5) }; // Accessing uninit memory here.
    let x = def + 1;
}
