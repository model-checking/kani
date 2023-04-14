// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
enum Niche_Enum {
    One(bool),
    Two(bool, bool),
}

enum Enum {
    One(u32),
    Two(u32, u32),
}

#[kani::proof]
fn check() {
    // This will have one operand.
    let _var = Niche_Enum::One(false);
    // This will have two operands -- true and false
    let _var = Niche_Enum::Two(true, false);
    // This will have one operand.
    let _var = Enum::One(1);
    // This will have two operands -- 2 and 3
    let _var = Enum::Two(2, 3);
}
