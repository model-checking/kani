// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that Kani detects the overflow in pointer offset

use std::convert::TryFrom;

struct Foo {
    arr: [i32; 4096],
}

#[cfg_attr(kani, kani::proof)]
fn main() {
    let f = Foo { arr: [0; 4096] };
    assert_eq!(std::mem::size_of::<Foo>(), 16384);
    // a large enough count that causes `count * size_of::<Foo>()` to overflow
    // `isize` without overflowing `usize`
    let count: usize = 562949953421320;
    // the `unwrap` ensures that it indeed doesn't overflow `usize`
    let bytes = count.checked_mul(std::mem::size_of::<Foo>()).unwrap();
    // ensure that it overflows `isize`:
    assert!(isize::try_from(bytes).is_err());

    let ptr: *const Foo = &f as *const Foo;
    // this should fail because `count * size_of::<Foo>` overflows `isize`
    let _p = unsafe { ptr.offset(count as isize) };
}
