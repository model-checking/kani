// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `assume` does not fail if the condition is true
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    unsafe { core::intrinsics::assume(true) };
}
