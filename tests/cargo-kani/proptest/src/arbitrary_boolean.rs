
// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! arbitrary boolean proptest

use proptest::bool;
use proptest::strategy::Just;

proptest::proptest! {
    fn arbitrary_boolean((_, (a,b)) in (Just(()), (bool::ANY, bool::ANY)) ) {
        assert!(a && b || true, "true shortcut");
    }
}
