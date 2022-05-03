// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    unsafe {
        std::intrinsics::unreachable();
    }
}
