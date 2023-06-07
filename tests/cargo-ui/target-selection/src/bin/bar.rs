// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Define a binary with a "bar" cover used to ensure pkg targets are correctly picked by Kani.

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn bar_harness() {
        kani::cover!(true, "Cover `bar`");
    }
}

fn main() {
    // Do nothing
}
