// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tuples of SVE scalable vectors, such as `svint32x4_t`, are laid out by rustc with a single
//! field offset however many vectors they hold. Kani's struct lowering asserted that each field
//! has an offset, so `core` failed to compile on aarch64, c.f.
//! <https://github.com/model-checking/kani/issues/4963>.
#![feature(stdarch_aarch64_sve)]

use std::arch::aarch64::*;

/// The harness builds the tuple field by field with `kani::any`.
#[target_feature(enable = "sve")]
pub fn pass_x2(t: svint32x2_t) -> svint32x2_t {
    t
}

#[target_feature(enable = "sve")]
pub fn pass_x4(t: svint32x4_t) -> svint32x4_t {
    t
}

/// Reading a member goes through the `sve_tuple_get` intrinsic, which Kani does not support.
#[target_feature(enable = "sve")]
pub fn first(t: svint32x4_t) -> svint32_t {
    svget4_s32::<0>(t)
}

/// The load is an LLVM intrinsic, which Kani does not support.
#[target_feature(enable = "sve")]
pub fn load(x: [i32; 16]) -> svint32x4_t {
    unsafe { svld4_s32(svptrue_b32(), x.as_ptr()) }
}
