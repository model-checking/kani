// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Define an integration test crate used to ensure pkg targets are correctly picked by Kani.

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn integ_harness() {
        kani::cover!(true, "Cover integration test");
    }
}
