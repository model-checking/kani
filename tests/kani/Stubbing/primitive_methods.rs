// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we can correctly stub primitive type functions.

/// Generate stub and harness for count_ones method on integers.
macro_rules! stub_count_ones {
    ($ty:ty, $harness:ident, $stub:ident) => {
        // Stub that always returns 0.
        pub fn $stub(_: $ty) -> u32 {
            0
        }

        // Harness
        #[kani::proof]
        #[kani::stub($ty::count_ones, $stub)]
        pub fn $harness() {
            let input = kani::any();
            let ones = <$ty>::count_ones(input);
            assert_eq!(ones, 0);
        }
    };
}

stub_count_ones!(u8, u8_count_ones, stub_u8_count_ones);
stub_count_ones!(u16, u16_count_ones, stub_u16_count_ones);
stub_count_ones!(u32, u32_count_ones, stub_u32_count_ones);
stub_count_ones!(u64, u64_count_ones, stub_u64_count_ones);
stub_count_ones!(u128, u128_count_ones, stub_u128_count_ones);
stub_count_ones!(usize, usize_count_ones, stub_usize_count_ones);

stub_count_ones!(i8, i8_count_ones, stub_i8_count_ones);
stub_count_ones!(i16, i16_count_ones, stub_i16_count_ones);
stub_count_ones!(i32, i32_count_ones, stub_i32_count_ones);
stub_count_ones!(i64, i64_count_ones, stub_i64_count_ones);
stub_count_ones!(i128, i128_count_ones, stub_i128_count_ones);
stub_count_ones!(isize, isize_count_ones, stub_isize_count_ones);

/// Check that we can stub is_ascii from `char`.
pub mod char_check {
    pub fn stub_is_ascii_true(_: &char) -> bool {
        true
    }

    #[kani::proof]
    #[kani::stub(char::is_ascii, stub_is_ascii_true)]
    pub fn check_stub_is_ascii() {
        let input: char = kani::any();
        assert!(input.is_ascii());
    }
}

/// Check that we can stub str::is_ascii
pub mod str_check {
    pub fn stub_is_ascii_false(_: &str) -> bool {
        false
    }

    #[kani::proof]
    #[kani::stub(str::is_ascii, stub_is_ascii_false)]
    pub fn check_stub_is_ascii() {
        let input = "is_ascii";
        assert!(!input.is_ascii());
    }
}

/// Check that we can stub slices
pub mod slices_check {

    #[derive(kani::Arbitrary)]
    pub struct MyStruct(u8, i32);

    pub fn stub_len_is_10<T>(v: &[T]) -> usize {
        10
    }

    #[kani::proof]
    #[kani::stub(<[MyStruct]>::len, stub_len_is_10)]
    pub fn check_stub_len_is_10() {
        let input: [MyStruct; 5] = kani::any();
        let slice = kani::slice::any_slice_of_array(&input);
        assert_eq!(slice.len(), 10);
    }
}

/// Check that we can stub raw pointer methods.
pub mod raw_ptr_check {
    pub fn stub_len_is_10<T>(_: *const [T]) -> usize {
        10
    }

    pub fn stub_mut_len_is_0<T>(_: *mut [T]) -> usize {
        0
    }

    #[kani::proof]
    #[kani::stub(<*const [u8]>::len, stub_len_is_10)]
    #[kani::stub(<*mut [u8]>::len, stub_mut_len_is_0)]
    pub fn check_stub_len_raw_ptr() {
        let mut input: [u8; 5] = kani::any();
        let mut_ptr = &mut input as *mut [u8];
        let ptr = &input as *const [u8];
        assert_eq!(mut_ptr.len(), 0);
        assert_eq!(ptr.len(), 10);
    }

    pub fn stub_is_always_null<T>(_: *const T) -> bool {
        true
    }

    // Fix-me: Option doesn't seem to work without the fully qualified path.
    #[kani::proof]
    #[kani::stub(<*const std::option::Option>::is_null, stub_is_always_null)]
    pub fn check_stub_is_null() {
        let input: Option<char> = kani::any();
        let ptr = &input as *const Option<char>;
        assert!(unsafe { ptr.is_null() });
    }
}

pub mod array_check {
    pub fn stub_as_slice_panic<T, const N: usize>(_: &[T; N]) -> &[T] {
        panic!("oops")
    }

    #[kani::proof]
    #[kani::should_panic]
    #[kani::stub(<[i32; 10]>::as_slice, stub_as_slice_panic)]
    pub fn check_stub_as_slice_panic() {
        let input: [i32; 10] = kani::any();
        let slice = input.as_slice();
        assert!(slice.get(0).is_some());
    }
}
