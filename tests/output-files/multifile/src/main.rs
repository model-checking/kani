// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub struct PrettyStruct;

#[kani::proof]
fn main() {
    pretty_function(PrettyStruct);
}

pub fn pretty_function(argument: PrettyStruct) -> PrettyStruct {
    argument
}
