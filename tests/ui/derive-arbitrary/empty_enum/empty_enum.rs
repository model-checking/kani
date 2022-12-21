// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive an empty Arbitrary enum but the method `panic!()`
//! when invoked since an empty enumeration cannot be instantiated.

#[derive(kani::Arbitrary)]
enum Empty {}

#[kani::proof]
fn check_no_variants() {
    let _e: Empty = kani::any();
    unreachable!();
}
