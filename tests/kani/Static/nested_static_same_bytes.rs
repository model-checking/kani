// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Nested statics with the same bytes must not share storage. Writing through
//! one static must not change the memory another static points to.
//! Several statics are used because codegen order follows symbol hashes.

macro_rules! nested {
    ($first:ident $($s:ident)*) => {
        static mut $first: &mut [u8; 4] = &mut [7, 7, 7, 7];
        $(static mut $s: &mut [u8; 4] = &mut [7, 7, 7, 7];)*

        #[kani::proof]
        fn check_nested_statics_not_shared() {
            unsafe {
                (*$first)[2] = 1;
                assert_eq!((*$first)[2], 1);
                $(assert_eq!(*$s, [7, 7, 7, 7]);)*
            }
        }
    };
}
nested!(N0 N1 N2 N3 N4 N5 N6 N7 N8 N9 N10 N11 N12 N13 N14 N15);
