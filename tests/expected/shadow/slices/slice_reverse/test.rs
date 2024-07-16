// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zghost-state

// This test demonstrates a possible usage of the shadow memory API to check that
// every element of a reversed array is initialized.
// Since the instrumentation is done manually in the harness only but not inside
// the `reverse` function, the test only verifies that the resulting array
// occupies the same memory as the original one.

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
