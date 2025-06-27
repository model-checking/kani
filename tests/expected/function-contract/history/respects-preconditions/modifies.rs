// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Demonstrate the the history expression respects preconditions
// with multiple interleaved preconditions, modifies contracts, and history expressions

#[derive(kani::Arbitrary)]
struct Point<X, Y> {
    x: X,
    y: Y,
}

#[kani::requires(ptr.x < 100)]
#[kani::ensures(|result| old(ptr.x + 1) == ptr.x)]
#[kani::modifies(&mut ptr.x)]
#[kani::ensures(|result| old(ptr.y - 1) == ptr.y)]
#[kani::modifies(&mut ptr.y)]
#[kani::requires(ptr.y > 0)]
fn modify(ptr: &mut Point<u32, u32>) {
    ptr.x += 1;
    ptr.y -= 1;
}

#[kani::proof_for_contract(modify)]
fn main() {
    let mut p: Point<u32, u32> = kani::any();
    modify(&mut p);
}
