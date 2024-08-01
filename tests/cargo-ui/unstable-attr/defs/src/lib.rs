// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::unstable_feature(feature = "always_fails", reason = "do not enable", issue = "<link>")]
pub fn always_fails() {
    assert!(false, "don't call me");
}

/// We use "gen-c" since it has to be an existing feature.
#[kani::unstable_feature(feature = "gen-c", reason = "internal fake api", issue = "<link>")]
pub fn no_op() {
    kani::cover!(true);
}
