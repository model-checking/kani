// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! add support for struct field projection for loop-contract

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

struct mystruct {
    a: i32,
    b: i32,
}

#[kani::proof]
fn struct_projection() {
    let mut s = mystruct { a: 0, b: 2 };
    let mut i = 0;
    #[kani::loop_invariant((i<=10) && (s.a == i) && (s.b == 2))]
    while i < 10 {
        s.a += 1;
        i += 1;
    }
    assert!(s.b == 2)
}
