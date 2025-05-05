// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that upcasting to a supertrait works correctly.

trait SubTrait: SuperTrait2 + SuperTrait1 {}

impl<T: SuperTrait1 + SuperTrait2> SubTrait for T {}

trait SuperTrait1 {
    fn trigger(&self, _old: &()) {}
}

trait SuperTrait2 {
    #[allow(unused)]
    fn func(&self) {}
}

#[derive(Clone, Copy, Default)]
struct Struct;

impl SuperTrait1 for Struct {}
impl SuperTrait2 for Struct {}

#[kani::proof]
fn main() {
    let val: &dyn SubTrait = &Struct;
    (val as &dyn SuperTrait1).trigger(&());
}
