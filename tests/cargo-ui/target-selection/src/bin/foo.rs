// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Define a binary with a "foo" cover used to ensure pkg targets are correctly picked by Kani.

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn foo_harness() {
        kani::cover!(true, "Cover `foo`");
    }
}

fn main() {
    // Do nothing
}
