// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zghost-state

// Check that every element of a reversed slice are initialized

const N: usize = 32;

static mut SM: kani::shadow::ShadowMem<bool> = kani::shadow::ShadowMem::new(false);

#[kani::proof]
fn check_reverse() {
    let mut a: [u16; N] = kani::any();
    for i in &a {
        unsafe { SM.set(i as *const u16, true) };
    }
    a.reverse();

    for i in &a {
        unsafe {
            assert!(SM.get(i as *const u16));
        }
    }
}
