// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles a call to an inherent method (`impl Counter {
//! fn get(&self) }`). Naming it used to ask the `impl` for its trait ref, which only trait impls
//! have, and aborted the compiler.

struct Counter {
    n: u32,
}

impl Counter {
    fn get(&self) -> u32 {
        self.n
    }
}

#[kani::proof]
fn main() {
    let c = Counter { n: 1 };
    let _ = c.get();
}
