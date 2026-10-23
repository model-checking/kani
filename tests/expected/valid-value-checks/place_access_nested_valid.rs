// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks

use std::mem::MaybeUninit;

#[repr(C)]
struct Inner {
    valid: bool,
    pending: bool,
}

#[repr(C)]
struct Outer {
    inner: Inner,
    other: bool,
}

#[kani::proof]
fn nested_initialized_field() {
    let mut storage = MaybeUninit::<Outer>::uninit();
    let ptr = storage.as_mut_ptr();
    unsafe {
        // Both sibling fields remain uninitialized throughout this harness.
        (&raw mut (*ptr).inner.valid).write(true);
        assert!((*ptr).inner.valid);
        let shared = &(*ptr).inner.valid;
        assert!(*shared);
        let mutable = &mut (*ptr).inner.valid;
        *mutable = false;
        assert!(!*mutable);
    }
    kani::cover!(true);
}
