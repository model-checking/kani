// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(core_intrinsics)]

// Reaching `abort_immediate` is modeled like `abort`: a panic-like failed
// assertion. A `should_panic` harness over an abort path therefore verifies.
// (This differs from `#[should_panic]` under `cargo test`, where an abort is
// not a panic and would terminate the test process instead.)
#[kani::proof]
#[kani::should_panic]
fn main() {
    core::intrinsics::abort_immediate();
}
