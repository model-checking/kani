// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Test that cargo kani works when there are ambiguous packages.
//! See <https://github.com/model-checking/kani/issues/3563>

use zerocopy::FromZeros;

#[kani::proof]
fn check_zero_copy() {
    let opt = Option::<&char>::new_zeroed();
    assert_eq!(opt, None);
}
