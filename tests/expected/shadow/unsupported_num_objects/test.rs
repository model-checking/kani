// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks the maximum number of objects supported by Kani's shadow
// memory model (currently 1024)

static mut SM: kani::shadow::ShadowMem = kani::shadow::ShadowMem::new();

fn check_max_objects<const N: usize>() {
    let mut i = 0;
    // A dummy loop that creates `N`` objects.
    // After the loop, CBMC's object ID counter should be at `N` + 2:
    // - `N` created in the loop +
    // - the NULL pointer whose object ID is 0, and
    // - the object ID for `i`
    while i < N {
        let x = i;
        assert_eq!(
            kani::mem::pointer_object(&x as *const usize as *const u8),
            i + 2
        );
        i += 1;
    }

    // create a new object whose ID is `N` + 2
    let x = 42;
    assert_eq!(
        kani::mem::pointer_object(&x as *const i32 as *const u8),
        N + 2
    );
    // the following call to `set_init` would fail if the object ID for `x`
    // exceeds the maximum allowed by Kani's shadow memory model
    unsafe {
        SM.set_init(&x as *const i32 as *const u8, true);
    }
}

#[kani::proof]
fn check_max_objects_pass() {
    check_max_objects::<1021>();
}

#[kani::proof]
fn check_max_objects_fail() {
    check_max_objects::<1022>();
}
