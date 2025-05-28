// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use dependency1;
use dependency2;

#[kani::proof]
fn harness() {
    assert!(dependency1::delegate_get_int() == 0);
    assert!(dependency2::delegate_get_int() == 1);

    assert!(dependency1::delegate_use_struct() == 3);
    assert!(dependency2::delegate_use_struct() == 1);
}

// Test that Kani can codegen repr(C) structs from two different versions of the same crate,
// c.f. https://github.com/model-checking/kani/issues/4007
#[kani::proof]
fn reprc_harness() {
    dependency1::create_struct();
    dependency2::create_struct();
}
