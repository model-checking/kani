// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness harness -Z stubbing
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

fn g2(_x: bool, _y: u32, _z: &mut bool, _zz: bool) -> i32 {
    42
}

fn h1<S>(_x: S) -> bool {
    true
}

fn h2<S, T>(_x: S) -> bool {
    true
}

fn i1<X: Copy, Y: Copy>(x: &X, _y: &Y) -> X {
    *x
}
fn i2<X: Copy, Y: Copy>(_x: &X, y: &Y) -> Y {
    *y
}

fn j1<X, Y>(_x: &X, _y1: &Y, _y2: &Y) {}

fn j2<X, Y>(_x1: &X, _x2: &X, _y: &Y) {}

#[kani::proof]
#[kani::stub(f1, f2)]
#[kani::stub(g1, g2)]
#[kani::stub(h1, h2)]
#[kani::stub(i1, i2)]
#[kani::stub(j1, j2)]
fn harness() {}
