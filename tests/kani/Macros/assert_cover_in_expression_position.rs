// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani's `assert!` and `cover!` macros can be used in expression
//! position (e.g. as `match` arms) without tripping the future-incompatible
//! `semicolon_in_expressions_from_non_local_macros` lint on the user's code.

#![deny(semicolon_in_expressions_from_non_local_macros)]

#[kani::proof]
fn check_macros_in_expression_position() {
    let x: u8 = kani::any();
    let _r = match x {
        0 => assert!(x == 0),
        _ => kani::cover!(x > 0),
    };
    let _s = if x > 3 { assert!(x > 3, "x is large") } else { kani::cover!() };
    let _t = if x == 7 { kani::cover!(x == 7, "seven") } else { assert!(x != 7) };
}
