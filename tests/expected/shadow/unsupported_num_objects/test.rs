// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zghost-state

// This test checks the maximum number of objects supported by Kani's shadow
// memory model (currently 1024)

static mut SM: kani::shadow::ShadowMem<bool> = kani::shadow::ShadowMem::new(false);

fn check_max_objects<const N: usize>() {
    let mut i = 0;
    // A dummy loop that creates `N`` objects.
    // After the loop, CBMC's object ID counter should be at `N` + 3:
    // - `N` created in the loop +
    // - the NULL pointer whose object ID is 0, and
    // - objects for i, have_42
    let mut have_42 = false;
    while i < N {
        let x: Box<usize> = Box::new(kani::any());
        if *x == 42 {
            have_42 = true;
        }
        i += 1;
    }

    // create a new object whose ID is `N` + 4
    let x: i32 = have_42 as i32;
    assert_eq!(x, have_42 as i32);
    // the following call to `set` would fail if the object ID for `x` exceeds
    // the maximum allowed by Kani's shadow memory model
    unsafe {
        SM.set(&x as *const i32, true);
    }
}

#[kani::proof]
fn check_max_objects_pass() {
    check_max_objects::<1019>();
}

#[kani::proof]
fn check_max_objects_fail() {
    check_max_objects::<1020>();
}
