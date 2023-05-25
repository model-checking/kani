// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that Kani handles harnesses with long name, e.g. due to
//! nested modules
//! The test is from https://github.com/model-checking/kani/issues/2468

mod a_really_long_module_name {
    mod yet_another_really_long_module_name {
        mod one_more_really_long_module_name {
            #[kani::proof]
            fn a_really_long_harness_name() {
                assert_eq!(1, 1);
            }
        }
    }
}
