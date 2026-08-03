// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that `kani::assert` produces a clean, user-facing
//! compiler error (instead of an internal compiler error / ICE) when its
//! message argument is not a string literal.
//!
//! Previously, this triggered an ICE at
//! kani-compiler/src/codegen_cprover_gotoc/overrides/hooks.rs:158 via
//! `gcx.extract_const_message(&msg).unwrap()`, because the message operand
//! was not reducible to a string literal at codegen time (here, it is a
//! function parameter rather than a literal expression at the call site).

fn assert_with_msg(cond: bool, msg: &'static str) {
    kani::assert(cond, msg);
}

#[kani::proof]
fn main() {
    assert_with_msg(true, "not actually a literal at the call site");
}
