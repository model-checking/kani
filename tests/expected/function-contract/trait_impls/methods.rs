// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
// kani-flags: -Zfunction-contracts

//! Check that we can add contract to methods and trait implementations.
//! Original code taken from:
//! <https://github.com/rust-lang/rust/blob/c4225812/library/core/src/ops/arith.rs#L1-L35>

use std::ops::Add;

#[derive(Debug, Copy, Clone, PartialEq, kani::Arbitrary)]
struct Point {
    x: i32,
    y: i32,
}

impl Add for Point {
    type Output = Self;

    #[kani::requires(!self.x.overflowing_add(other.x).1)]
    #[kani::requires(!self.y.overflowing_add(other.y).1)]
    #[kani::ensures(|result| result.x == self.x + other.x)]
    #[kani::ensures(|result| result.y == self.y + other.y)]
    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

impl Point {
    #[kani::modifies(&mut self.x)]
    #[kani::requires(!self.x.overflowing_add(val).1)]
    #[kani::ensures(|_| val < 0 || self.x >= old(self.x))]
    #[kani::ensures(|_| val > 0 || self.x <= old(self.x))]
    pub fn add_x(&mut self, val: i32) {
        self.x += val;
    }

    #[kani::requires(self.y < i32::MAX)]
    #[kani::ensures(|result| result.y == old(self.y) + 1)]
    pub fn next_y(mut self) -> Self {
        self.y += 1;
        self
    }
}

#[kani::proof_for_contract(Point::add_x)]
fn check_add_x() {
    let mut p1: Point = kani::any();
    let _ = p1.add_x(kani::any());
}

#[kani::proof_for_contract(Point::next_y)]
fn check_next_y() {
    let p1: Point = kani::any();
    let _ = p1.next_y();
}

/// We should enable this once we add support to specifying trait methods:
/// <https://github.com/model-checking/kani/issues/1997>
#[cfg(ignore)]
#[kani::proof_for_contract(Point::add)]
fn check_add() {
    let (p1, p2): (Point, Point) = kani::any();
    let _ = p1.add(p2);
}
