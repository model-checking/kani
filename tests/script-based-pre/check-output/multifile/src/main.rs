// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Add a dummy data so PrettyStruct doesn't get removed from arg list.
pub struct PrettyStruct(u32);

#[kani::proof]
fn main() {
    pretty_function(PrettyStruct(2));
}

pub fn pretty_function(argument: PrettyStruct) -> PrettyStruct {
    argument
}
