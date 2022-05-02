// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `ceilf32` is not supported until
// https://github.com/model-checking/kani/issues/1025 is fixed
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let x = kani::any();
    let x_ceil = unsafe { std::intrinsics::ceilf32(x) };
}
