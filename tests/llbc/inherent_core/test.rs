// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles a call to an inherent method on a primitive
//! from `core` (`u8::wrapping_add`, defined in `impl u8`), which aborted the compiler the same way
//! as `inherent_local`. The method name is suffixed with the implementing type, as for trait impl
//! methods.

fn wrap(a: u8, b: u8) -> u8 {
    a.wrapping_add(b)
}

#[kani::proof]
fn main() {
    let _ = wrap(200, 100);
}
