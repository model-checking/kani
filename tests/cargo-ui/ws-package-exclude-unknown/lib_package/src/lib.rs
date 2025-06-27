// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub fn api() {}

#[kani::proof]
fn harness_in_lib_package() {
    assert!(1 + 1 == 2);
}
