// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --verbose --only-codegen
//
//! Checks that we print compilation stats when we pass `--verbose`

use std::num::NonZeroU8;

fn non_zero(x: u8) {
    assert!(x != 0);
}

#[kani::proof]
fn check_variable() {
    non_zero(kani::any::<NonZeroU8>().into());
}
