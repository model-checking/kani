// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Proptests with a single primitive strategy. Also expanded for
//! clarity during testing.

kani::translate_from_proptest!{
proptest! {
    #[kani::proof]
    fn proptest_u8 (input_ in proptest::arbitrary::any<u8>) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_u16 (input_ in proptest::arbitrary::any<u16>) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_u32 (input_ in proptest::arbitrary::any<u32>) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_u64 (input_ in proptest::arbitrary::any<u64>) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_u128 (input_ in proptest::arbitrary::any<u128>) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_usize (input_ in proptest::arbitrary::any<usize>) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i8 (input_ in proptest::arbitrary::any<i8>) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i16 (input_ in proptest::arbitrary::any<i16>) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i32 (input_ in proptest::arbitrary::any<i32>) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i64 (input_ in proptest::arbitrary::any<i64>) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i128 (input_ in proptest::arbitrary::any<i128>) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_isize (input_ in proptest::arbitrary::any<isize>) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}
}
