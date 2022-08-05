// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A proptest that uses arbitrary "any" function

use proptest::arbitrary::any;
use proptest::bool;

proptest::proptest! {
    fn arbitrary_boolean((_, (a,b)) in (any::<()>(), any::<(bool, bool)>()) ) {
        assert!( (a && b) || true, "true shortcut");
    }
}
