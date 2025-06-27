// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test case checks the behavior of `align_of_val_raw` for dynamically sized types.

#![feature(layout_for_ptr)]
#![feature(ptr_metadata)]

use std::any::Any;
use std::mem::align_of;

#[allow(unused)]
#[derive(Clone, Copy, kani::Arbitrary)]
struct Wrapper<T, U: ?Sized> {
    head: T,
    dst: U,
}

/// Create a ZST with an alignment of 1024.
#[allow(unused)]
#[repr(align(1024))]
#[derive(kani::Arbitrary)]
struct Zst1024;

/// Create a structure with large alignment (2^29).
///
/// This seems to be the maximum supported today:
/// <https://github.com/rust-lang/rust/blob/7db7489f9bc2/tests/ui/repr/repr-align.rs#L11>
#[repr(align(536870912))]
#[derive(kani::Arbitrary)]
struct LargeAlign {
    data: [usize; 100],
}

/// Generates a harness with different type combinations and check the alignment is correct.
/// We use a constant for the original type, so it is pre-populated by the compiler.
///
/// ## Parameters
/// - `name`: Name of the harness.
/// - `t1` / `t2`: Types used for different tail and head combinations.
/// - `expected`: The expected alignment.
macro_rules! check_alignment {
    ($name:ident, $t1:ty, $t2:ty, $expected:expr) => {
        #[kani::proof]
        fn $name() {
            const EXPECTED: usize = align_of::<Wrapper<$t1, $t2>>();
            assert_eq!(EXPECTED, $expected);

            let var: Wrapper<$t1, $t2> = kani::any();
            let wide_ptr: &Wrapper<$t1, dyn Any> = &var as &_;
            let dyn_t2_align = align_of_val(wide_ptr);
            assert_eq!(dyn_t2_align, EXPECTED, "Expected same alignment as before coercion");

            let var: Wrapper<$t2, $t1> = kani::any();
            let wide_ptr: &Wrapper<$t2, dyn Any> = &var as &_;
            let dyn_t1_align = align_of_val(wide_ptr);
            assert_eq!(dyn_t1_align, EXPECTED, "Expected same alignment as before coercion");

            let var: Wrapper<$t1, [$t2; 0]> = kani::any();
            let wide_ptr: &Wrapper<$t1, [$t2]> = &var as &_;
            let slice_t2_align = align_of_val(wide_ptr);
            assert_eq!(slice_t2_align, EXPECTED, "Expected same alignment as before coercion");

            let var: Wrapper<$t2, [$t1; 0]> = kani::any();
            let wide_ptr: &Wrapper<$t2, [$t1]> = &var as &_;
            let slice_t1_align = align_of_val(wide_ptr);
            assert_eq!(slice_t1_align, EXPECTED, "Expected same alignment as before coercion");
        }
    };
}

check_alignment!(check_1zst_usize, usize, (), align_of::<usize>());
check_alignment!(check_1char_tup, (char, usize, u128), char, align_of::<u128>());
check_alignment!(check_zst1024, (char, usize, u128), Zst1024, align_of::<Zst1024>());
check_alignment!(check_large, u128, LargeAlign, align_of::<LargeAlign>());
