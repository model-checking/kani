// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Define a library with a lib cover used to ensure pkg targets are correctly picked by Kani.

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn lib_harness() {
        kani::cover!(true, "Cover lib");
    }
}
