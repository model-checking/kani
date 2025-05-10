// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that Kani can codegen statics that contain pointers to nested statics.
// See https://github.com/model-checking/kani/issues/3904

mod example_1 {
    // FOO contains a pointer to the anonymous nested static alloc2.
    // The MIR is:
    // alloc1 (static: FOO, size: 8, align: 8) {
    //     ╾───────alloc2────────╼                         │ ╾──────╼
    // }

    // alloc2 (static: FOO::{constant#0}, size: 4, align: 4) {
    //     2a 00 00 00                                     │ *...
    // }
    static mut FOO: &mut u32 = &mut 42;

    #[kani::proof]
    fn main() {
        unsafe {
            *FOO = 43;
        }
    }
}

mod example_2 {
    // FOO and BAR both point to the anonymous nested static alloc2.
    // The MIR is:
    // alloc3 (static: BAR, size: 8, align: 8) {
    //     ╾───────alloc2────────╼                         │ ╾──────╼
    // }

    // alloc2 (static: FOO::{constant#0}, size: 4, align: 4) {
    //     2a 00 00 00                                     │ *...
    // }

    // alloc1 (static: FOO, size: 8, align: 8) {
    //     ╾───────alloc2────────╼                         │ ╾──────╼
    // }

    static mut FOO: &mut i32 = &mut 12;
    static mut BAR: *mut i32 = unsafe { FOO as *mut _ };

    #[kani::proof]
    fn main() {
        unsafe {
            // check that we see the same initial value from all aliases
            assert_eq!(*FOO, 12);
            assert_eq!(*BAR, 12);
            *FOO = 13;
            // check that we see the same mutated value from all aliases
            assert_eq!(*FOO, 13);
            assert_eq!(*BAR, 13);
        }
    }
}
