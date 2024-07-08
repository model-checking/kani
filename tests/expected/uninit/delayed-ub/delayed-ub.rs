// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

/// Checks that Kani rejects mutable pointer casts between types of different padding.
#[kani::proof]
fn invalid_value() {
    unsafe {
        let mut value: u128 = 0;
        let ptr = &mut value as *mut _ as *mut (u8, u32, u64);
        *ptr = (4, 4, 4); // This assignment itself does not cause UB...
        let c: u128 = value; // ...but this reads a padding value!
    }
}
