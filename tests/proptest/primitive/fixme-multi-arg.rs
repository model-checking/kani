// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Proptests with a single primitive input. This repetition should be
//! done with macros, but since we have already 2 nested macros, I
//! pre-expanded for clarity during development.

kani::translate_from_proptest!{
proptest! {
    #[kani::proof]
    fn proptest_two_types(input_1 : u8, input_2 : i32) {
        let derived = input_2 << input_1;
	assert!(derived + derived >= 0);
	assert_eq!(derived - derived, 0);
    }

    #[kani::proof]
    fn proptest_two_strategies(input_1 : proptest::arbitrary::any<u8>, input_2 : proptest::arbitrary::any<i32>) {
        let derived = input_2 << input_1;
	assert!(derived + derived >= 0);
	assert_eq!(derived - derived, 0);
    }

    #[kani::proof]
    fn proptest_two_mixed_type_first(input_1 : u8, input_2 : proptest::arbitrary::any<i32>) {
        let derived = input_2 << input_1;
	assert!(derived + derived >= 0);
	assert_eq!(derived - derived, 0);
    }

    #[kani::proof]
    fn proptest_two_mixed_type_first(input_1 : proptest::arbitrary::any<u8>, input_2 : i32) {
        let derived = input_2 << input_1;
	assert!(derived + derived >= 0);
	assert_eq!(derived - derived, 0);
    }
}
}

