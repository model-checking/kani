// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This tests to make sure that we do not resolve paths that must be local
//! -- because they begin with `self` etc. or because their first segment
//! matches the name of a local module -- to external functions that match that
//! path.

use other_crate1;
use other_crate2;

mod my_mod {

    mod other_crate1 {}

    fn orig1() {}

    fn orig2() {}

    fn orig3() {}

    fn orig4() {}

    #[kani::proof]
    // We should not resolve `other_crate1::mock` to the external function (with
    // that name) because there is a module here named `other_crate1` (which
    // shadows the crate during path resolution).
    #[kani::stub(orig1, other_crate1::mock)]
    // We should not resolve any of these stubs to `other_crate2::mock` because
    // the paths begin with local qualifiers (`self`, etc.).
    #[kani::stub(orig2, self::other_crate2::mock)]
    #[kani::stub(orig3, super::other_crate2::mock)]
    #[kani::stub(orig4, crate::other_crate2::mock)]
    fn harness() {
        assert!(false);
    }
}
