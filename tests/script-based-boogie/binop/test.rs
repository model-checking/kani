// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A simple test that checks that binary operations are translated to a
//! function call to an SMT bv operator
//!
//! The `expected` file checks some elements of the expected output Boogie.
//!
//! This is a possible Boogie output (which may change with MIR changes):
//! ```
//! function {:bvbuiltin "bvslt"} $BvSignedLessThan<T>(lhs: T, rhs: T) returns (bool);
//!
//! function {:bvbuiltin "bvsgt"} $BvSignedGreaterThan<T>(lhs: T, rhs: T) returns (bool);
//!
//! function {:bvbuiltin "bvor"} $BvOr<T>(lhs: T, rhs: T) returns (T);
//!
//! // Procedures:
//! procedure _RNvCshNorBqfmMTU_4test14check_binop_gt()
//! {
//!   var _1: bv32;
//!   var _2: bv32;
//!   var _4: bool;
//!   var z: bv32;
//!   var _7: bool;
//!   _1 := 1bv32;
//!   _2 := 2bv32;
//!   _4 := $BvSignedGreaterThan(_2, _1);
//!   assert _4;
//!   z := $BvAnd(_1, _2);
//!   _7 := $BvSignedLessThan(_1, z);
//!   assert _7;
//!   return;
//! }
//!
//! ````

#[kani::proof]
fn check_binop_gt() {
    let x = 1;
    let y = 2;
    kani::assert(y > x, "");
    let z = x | y;
    kani::assert(x < z, "");
}
