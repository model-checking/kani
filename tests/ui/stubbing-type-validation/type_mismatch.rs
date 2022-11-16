// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness harness --enable-unstable --enable-stubbing
//
//! This tests that we catch type mismatches between the stub and the original
//! function/method.

fn f1(_x: i32) -> bool {
    true
}

fn f2() -> bool {
    true
}

fn g1(_x: bool, _y: i32, _z: &bool, _zz: bool) -> bool {
    true
}

fn g2(_x: bool, _y: u32, _z: &mut bool, _zz: bool) -> bool {
    true
}

fn h1<S>(_x: S) -> bool {
    true
}

fn h2<T>(_x: T) -> bool {
    true
}

#[kani::proof]
#[kani::stub(f1, f2)]
#[kani::stub(g1, g2)]
#[kani::stub(h1, h2)]
fn harness() {}
