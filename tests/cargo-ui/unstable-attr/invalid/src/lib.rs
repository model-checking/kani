// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! All the unstable definitions below should fail.
//! The expected file only contains a generic check since we trigger an ICE for debug builds and
//! we don't guarantee the order that these will be evaluated.
//! TODO: We should break down this test to ensure all of these fail.

#[kani::unstable_feature(reason = "just checking", issue = "<link>")]
pub fn missing_feature() {
    todo!()
}

#[kani::unstable_feature(feature("invalid_args"))]
pub fn invalid_fn_style() {}

#[kani::unstable_feature(feature, issue)]
pub fn invalid_list() {}

#[kani::unstable_feature(1010)]
pub fn invalid_argument() {}

#[kani::proof]
pub fn harness() {
    missing_feature();
    invalid_fn_style();
    invalid_list();
    invalid_argument();
}
