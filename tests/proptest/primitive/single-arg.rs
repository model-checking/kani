// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Proptests with a single primitive input

macro_rules! unsigned_single_input_proptest {
    ($fn_name:ident, $type:ty) => {
	kani::proptest! {
	    #[kani::proof]
	    fn $fn_name (input_1 : $type) {
		assert!(input_1 + input_1 >= 0);
		assert_eq!(input_1 - input_1, 0);
	    }
	}
    };
}

unsigned_single_input_proptest!(proptest_u8, u8);
unsigned_single_input_proptest!(proptest_u16, u16);
unsigned_single_input_proptest!(proptest_u32, u32);
unsigned_single_input_proptest!(proptest_u64, u64);
unsigned_single_input_proptest!(proptest_u128, u128);
unsigned_single_input_proptest!(proptest_usize, usize);

macro_rules! signed_single_input_proptest {
    ($fn_name:ident, $type:ty) => {
	kani::proptest! {
	    #[kani::proof]
	    fn $fn_name (input_1 : $type) {
		assert!(input_1 * input_1 >= 0);
		assert_eq!(input_1 - input_1, 0);
	    }
	}
    };
}

signed_single_input_proptest!(proptest_i8, i8);
signed_single_input_proptest!(proptest_i16, i16);
signed_single_input_proptest!(proptest_i32, i32);
signed_single_input_proptest!(proptest_i64, i64);
signed_single_input_proptest!(proptest_i128, i128);
signed_single_input_proptest!(proptest_isize, isize);
