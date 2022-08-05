// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! cargo-kani test crate for proptest.

mod arbitrary_boolean;

use proptest::test_runner::Config;

// check if the proptest library is linked and macro is working.
proptest::proptest! {
    fn successfully_linked_proptest(_ in proptest::strategy::Just(()) ) {
        let config = Config::default();
        assert_eq!(
            config.cases,
            256,
            "Default .cases should be 256. Check library/proptest/src/test_runner/config.rs"
        );
    }
}
