// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A simple test that checks that assignment statements and asserts are
//! translated to Boogie correctly.
//!
//! The `expected` file checks some elements of the expected output Boogie.
//!
//! This is a possible Boogie output (which may change with MIR changes):
//! ```
//! // Procedures:
//! procedure _RNvCshNorBqfmMTU_4test19check_assign_assert()
//! {
//!   var x: bv32;
//!   var y: bv32;
//!   var _4: bool;
//!   var _5: bv32;
//!   var _7: bool;
//!   x := 1bv32;
//!   y := x;
//!   x := 2bv32;
//!   _5 := x;
//!   _4 := (_5 == 2bv32);
//!   assert _4;
//!   _7 := (y == 1bv32);
//!   assert _7;
//!   return;
//! }
//! ```

#[kani::proof]
fn check_assign_assert() {
    let mut x = 1;
    let y = x;
    x = 2;
    kani::assert(x == 2, "");
    kani::assert(y == 1, "");
}
