// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

#[kani::proof]
fn main() {
    let mut v: Vec<u8> = Vec::with_capacity(10);
    unsafe {
        v.set_len(5);
    }
    // We would read uninitialized values on drop, but MIRI doesn't seem to complain about it either.
}
