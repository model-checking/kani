// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
// kani-flags: -Zfunction-contracts

//! Check that we can add contract to a trait implementation.
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

#[kani::proof_for_contract(add)]
fn check_add() {
    let (p1, p2): (Point, Point) = kani::any();
    p1.add(p2);
}
