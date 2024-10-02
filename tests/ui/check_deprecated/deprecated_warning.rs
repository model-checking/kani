// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --only-codegen

/// Ensure that Kani prints a deprecation warning if users invoke `kani::check`.
#[kani::proof]
fn internal_api() {
    kani::check(kani::any(), "oops");
}
