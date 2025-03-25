// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z mem-predicates
#![feature(ptr_metadata)]

extern crate kani;

mod size {    
    use super::*;

    #[kani::proof]
    fn verify_checked_size_of_raw_exceeds_isize_max() {
        let len_exceeding_isize_max = (isize::MAX as usize) + 1;
        let data_ptr: *const [u8] = core::ptr::from_raw_parts(core::ptr::null::<u8>(), len_exceeding_isize_max);

        let size = kani::mem::checked_size_of_raw(data_ptr);

        assert!(size.is_none());
    }

    #[kani::proof]
    fn verify_is_inbounds() {
        let val = 42u8;
        let ptr = &val as *const u8;
        assert!(kani::mem::is_inbounds(ptr));

        let null_ptr: *const u8 = core::ptr::null();
        assert!(!kani::mem::is_inbounds(null_ptr));
    }
}