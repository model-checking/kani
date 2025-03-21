// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that Kani can codegen statics that contain pointers to nested statics.
// See https://github.com/model-checking/kani/issues/3904

mod example_1 {
    // FOO contains a pointer to the anonymous nested static alloc2.
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
    // Test taken from https://github.com/rust-lang/rust/issues/71212#issuecomment-738666248
    // alloc3 (static: BAR, size: 16, align: 8) {
    //     ╾───────alloc2────────╼ 01 00 00 00 00 00 00 00 │ ╾──────╼........
    // }

    // alloc2 (static: FOO::{constant#0}, size: 4, align: 4) {
    //     2a 00 00 00                                     │ *...
    // }

    // alloc1 (static: FOO, size: 16, align: 8) {
    //     ╾───────alloc2────────╼ 01 00 00 00 00 00 00 00 │ ╾──────╼........
    // }
    pub mod a {
        #[no_mangle]
        pub static mut FOO: &mut [i32] = &mut [42];
    }

    pub mod b {
        #[no_mangle]
        pub static mut BAR: &mut [i32] = unsafe { &mut *crate::example_2::a::FOO };
    }

    #[kani::proof]
    fn main() {
        unsafe {
            assert_eq!(a::FOO.as_ptr(), b::BAR.as_ptr());
        }
    }
}
