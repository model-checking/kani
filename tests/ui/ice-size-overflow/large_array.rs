// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that the compiler handles extremely large array sizes
//! gracefully with a proper error message instead of causing an ICE.
//! Previously, this would trigger an internal compiler error at
//! kani-compiler/src/codegen_cprover_gotoc/context/goto_ctx.rs:371:9

struct S<T> {
    x: [T; !0],
}

pub fn f() -> usize {
    std::mem::size_of::<S<u8>>()
}

#[kani::proof]
fn main() {
    let _x = f();
}
