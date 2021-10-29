// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Currently fails with thread 'rustc' panicked at 'assertion failed: w < 128', compiler/rustc_codegen_llvm/src/gotoc/cbmc/goto_program/typ.rs:508:9

#![feature(core_intrinsics)]
use std::intrinsics;

pub fn main() {
    let v: u128 = rmc::nondet();
    let w: u128 = rmc::nondet();
    intrinsics::saturating_add(v, w);
}
