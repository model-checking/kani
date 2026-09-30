// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles a call to a function that
//! never returns. Such a call has no return target in MIR, which used to make
//! the backend panic while translating the `Call` terminator.

fn diverge() -> ! {
    loop {}
}

#[kani::proof]
fn main() {
    diverge();
}
