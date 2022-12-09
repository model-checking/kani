// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance of pushing onto a BTreeSet
//! The test is from https://github.com/model-checking/kani/issues/705.
//! Pre CBMC 5.72.0, it ran out of memory
//! With CBMC 5.72.0, it takes ~10 seconds and consumes ~255 MB of memory.

use std::collections::BTreeSet;

#[kani::proof]
#[kani::unwind(3)]
fn main() {
    let mut set = BTreeSet::<u32>::new();
    let x = kani::any();
    set.insert(x);
    assert!(set.contains(&x));
}
