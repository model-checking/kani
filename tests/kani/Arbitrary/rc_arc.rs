// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that Rc<T> and Arc<T> implement Arbitrary (like Box<T>), producing an
// unconstrained value of the pointee type behind shared-ownership indirection.

use std::rc::Rc;
use std::sync::Arc;

#[derive(kani::Arbitrary)]
struct Point {
    x: u8,
    y: u8,
}

#[kani::proof]
fn check_rc() {
    let p: Rc<Point> = kani::any();
    kani::cover!(p.x == 255 && p.y == 0, "extreme values are generated");
}

#[kani::proof]
fn check_arc() {
    let v: Arc<i32> = kani::any();
    kani::cover!(*v == i32::MIN, "extreme values are generated");
    kani::cover!(*v == 42, "specific values are generated");
}

#[kani::proof]
fn check_nested() {
    let v: Rc<Arc<u8>> = kani::any();
    kani::cover!(**v == 7, "nested smart pointers are generated");
}
