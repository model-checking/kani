// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// A no_std crate that denies unused_crate_dependencies used to fail under Kani, since Kani's
// injected `--extern noprelude:std` (which such a crate never references) tripped the lint.
// The `nounused` extern modifier exempts the injected extern while keeping the lint active
// for the crate's real dependencies.

#![no_std]
#![deny(unused_crate_dependencies)]

#[cfg(kani)]
extern crate kani;

fn add(x: u8, y: u8) -> u8 {
    x.wrapping_add(y)
}

#[cfg(kani)]
#[kani::proof]
fn check_add() {
    let x: u8 = kani::any();
    let y: u8 = kani::any();
    assert!(add(x, y) == x.wrapping_add(y));
}
