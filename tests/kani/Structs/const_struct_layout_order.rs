// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Regression test for an ICE in `try_codegen_constant`. For a constant struct
//! with a single non-ZST field it built the field values in declaration order
//! and assigned them positionally, but the goto struct type lists fields in
//! LAYOUT order. When layout reorders the fields, the positional assignment
//! paired a value with the wrong field slot, causing
//! "value type does not match field type".
//!
//! Here the zero-sized `Marker` is declared before the `u64` `value`, but
//! because `u64` has the larger alignment the compiler places it first in
//! memory, so layout order differs from declaration order. The struct has
//! exactly one non-ZST field, which is required to reach the affected code
//! path (`try_codegen_constant`'s direct struct expansion is gated on there
//! being a single non-ZST field).

struct Marker;

struct Reordered {
    marker: Marker,
    value: u64,
}

const R: Reordered = Reordered { marker: Marker, value: 512 };

#[kani::proof]
fn check_const_struct_layout_order() {
    let r = R;
    // Touch the ZST field so it is not optimized away entirely.
    let Marker = r.marker;
    assert!(r.value == 512);
}
