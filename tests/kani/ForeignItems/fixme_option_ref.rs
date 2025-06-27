// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z c-ffi --c-lib tests/kani/ForeignItems/lib.c
//! Check how Kani handles declaration of Nullable reference (Option<&T>).
//! See <https://github.com/model-checking/kani/issues/2152> for more details.

extern "C" {
    // In Rust, you say nullable pointer by using option of reference.
    // Rust guarantees that this has the bitwise representation:
    //  - Some(&x) => &x;
    //  - None => NULL;
    // FIXME: we need to notice when this happens and do a bitcast, or C is unhappy
    // <https://github.com/model-checking/kani/issues/3>
    fn takes_ptr_option(p: Option<&u32>) -> u32;
}

#[kani::proof]
fn main() {
    unsafe {
        // if (ptr) { *ptr - 1 }
        assert!(takes_ptr_option(Some(&5)) == 4);
        // if (!ptr) { 0 }
        assert!(takes_ptr_option(None) == 0);
    }
}
