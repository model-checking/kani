// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This is just to test that cargo kani --debug works.

#[cfg(kani)]
mod harnesses {
    #[kani::proof]
    fn harness() {}
}
