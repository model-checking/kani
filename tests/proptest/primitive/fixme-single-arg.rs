// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Proptests with a single primitive input. This repetition should be
//! done with macros, but since we have already 2 nested macros, I
//! pre-expanded for clarity.

kani::translate_from_proptest!{
proptest! {
    #[kani::proof]
    fn proptest_u8 (input_1 : u8) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_u16 (input_1 : u16) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_u32 (input_1 : u32) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_u64 (input_1 : u64) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_u128 (input_1 : u128) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_usize (input_1 : usize) {
	assert!(input_1 + input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i8 (input_1 : i8) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i16 (input_1 : i16) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i32 (input_1 : i32) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i64 (input_1 : i64) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_i128 (input_1 : i128) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}

proptest! {
    #[kani::proof]
    fn proptest_isize (input_1 : isize) {
	assert!(input_1 * input_1 >= 0);
	assert_eq!(input_1 - input_1, 0);
    }
}
}
