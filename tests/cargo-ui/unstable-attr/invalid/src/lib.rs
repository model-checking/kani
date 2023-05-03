// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::unstable(reason = "just checking", issue = "<link>")]
pub fn missing_feature() {
    todo!()
}

#[kani::unstable(feature("invalid_args"))]
pub fn invalid_fn_style() {}

#[kani::unstable(feature, issue)]
pub fn invalid_list() {}

#[kani::unstable(1010)]
pub fn invalid_argument() {}

#[kani::proof]
pub fn harness() {
    missing_feature();
    invalid_fn_style();
    invalid_list();
    invalid_argument();
}
