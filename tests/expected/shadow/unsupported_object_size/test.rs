// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks the maximum object size supported by Kani's shadow
// memory model (currently 64)

static mut SM: kani::shadow::ShadowMem = kani::shadow::ShadowMem::new();

fn check_max_objects<const N: usize>() {
    let arr: [u8; N] = [0; N];
    let last = &arr[N - 1];
    assert_eq!(kani::mem::pointer_offset(last as *const u8), N - 1);
    // the following call to `set_init` would fail if the object offset for
    // `last` exceeds the maximum allowed by Kani's shadow memory model
    unsafe {
        SM.set_init(last as *const u8, true);
    }
}

#[kani::proof]
fn check_max_object_size_pass() {
    check_max_objects::<64>();
}

#[kani::proof]
fn check_max_object_size_fail() {
    check_max_objects::<65>();
}
