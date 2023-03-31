// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `#[kani::should_panic]` can only be used once.

#[kani::proof]
#[kani::should_panic]
#[kani::should_panic]
fn check() {}
