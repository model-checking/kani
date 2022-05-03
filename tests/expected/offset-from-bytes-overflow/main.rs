// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that an offset computed with `offset_from` triggers a verification failure
// if it overflows an `isize` in bytes.
use std::convert::TryInto;

#[kani::proof]
fn main() {
    let v: &[u128] = &[0; 200];
    let v_100: *const u128 = &v[100];
    let max_offset = usize::MAX / std::mem::size_of::<u128>();
    unsafe {
        let v_wrap: *const u128 = v_100.add((max_offset + 1).try_into().unwrap());
        let _ = v_wrap.offset_from(v_100) == 2;
    }
}
