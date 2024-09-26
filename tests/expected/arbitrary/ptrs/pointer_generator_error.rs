// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z mem-predicates
//! Check misusage of pointer generator fails compilation.
extern crate kani;

use kani::PointerGenerator;

pub fn check_invalid_generator() {
    let _generator = PointerGenerator::<0>::new();
}
