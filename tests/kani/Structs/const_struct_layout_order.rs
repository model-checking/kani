// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Constant structs whose layout order differs from declaration order must codegen with
//! name-keyed fields (regression: value/field type mismatch ICE in try_codegen_constant).
//! `Pair` declares (u8-wrapper, u16); layout places the u16 first.

#[derive(Clone, Copy, PartialEq)]
struct Wrap(u8);

#[derive(Clone, Copy, PartialEq)]
struct Pair {
    w: Wrap,
    n: u16,
}

const P: Pair = Pair { w: Wrap(3), n: 512 };

#[kani::proof]
fn check_const_struct_layout_order() {
    let p = P;
    assert!(p.w == Wrap(3));
    assert!(p.n == 512);
}
