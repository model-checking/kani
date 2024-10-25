// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing

//! Kani supports stubbing of most primitive methods, however, some methods, such as len, may be
//! lowered to an Rvalue.

/// Check that we can stub slices methods
pub mod slices_check {
    #[derive(kani::Arbitrary)]
    pub struct MyStruct(u8, i32);

    pub fn stub_len_is_10<T>(_: &[T]) -> usize {
        10
    }

    // This fails since `<[T]>::len` is lowered to `Rvalue::Len`.
    #[kani::proof]
    #[kani::stub(<[MyStruct]>::len, stub_len_is_10)]
    pub fn check_stub_len_is_10() {
        let input: [MyStruct; 5] = kani::any();
        let slice = kani::any_slice_of_array(&input);
        assert_eq!(slice.len(), 10);
    }
}
