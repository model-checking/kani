// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zghost-state

// Check that every element of a slice split into two is initialized

const N: usize = 16;

static mut SM: kani::shadow::ShadowMem<bool> = kani::shadow::ShadowMem::new(false);

#[kani::proof]
#[kani::unwind(17)]
fn check_reverse() {
    let a: [bool; N] = kani::any();
    for i in &a {
        unsafe { SM.set(i as *const bool, true) };
    }
    let index: usize = kani::any_where(|x| *x <= N);
    let (s1, s2) = a.split_at_checked(index).unwrap();

    for i in s1 {
        unsafe {
            assert!(SM.get(i as *const bool));
        }
    }
    for i in s2 {
        unsafe {
            assert!(SM.get(i as *const bool));
        }
    }
}
