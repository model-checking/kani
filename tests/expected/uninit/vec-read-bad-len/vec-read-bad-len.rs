// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

use std::ops::Index;

#[kani::proof]
fn main() {
    let mut v: Vec<u8> = Vec::with_capacity(10);
    unsafe {
        v.set_len(5);
    }
    let el = v.index(0);
}
