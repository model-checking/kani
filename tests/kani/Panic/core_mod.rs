// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that the panic macro does not cause a compiler error if
//! there is a user-defined `core` module, which previously did (see
//! https://github.com/model-checking/kani/issues/1984)

mod core {}

#[kani::proof]
fn main() {
    let x: u8 = kani::any();
    let y = x / 2;
    if y > x {
        // impossible
        panic!("y is {}", y);
    }
}
