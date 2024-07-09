// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
// kani-flags: -Zfunction-contracts

//! Check that we can add contract to a trait implementation.
//! Original code taken from:
//! <https://github.com/rust-lang/rust/blob/c4225812/library/core/src/ops/arith.rs#L1-L35>
//!
//! TODO: Add the following tests
//! Multiple annotations and:
//!  - mut args
//!  - inner functions

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
    #[kani::ensures(|_| val > 0 || self.x < old(self.x))]
    #[kani::ensures(|_| val < 0 || self.x > old(self.x))]
    pub fn add_x(&mut self, val: i32) {
        self.x += val;
    }
}

#[kani::proof_for_contract(add)]
fn check_add() {
    let (p1, p2): (Point, Point) = kani::any();
    let _ = p1.add(p2);
}
