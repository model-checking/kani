// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Currently fails with thread 'rustc' panicked at 'assertion failed: w < 128', compiler/rustc_codegen_llvm/src/gotoc/cbmc/goto_program/typ.rs:508:9

#![feature(core_intrinsics)]
use std::intrinsics;

fn main() {
    let v: u128 = rmc::any();
    let w: u128 = rmc::any();
    intrinsics::saturating_add(v, w);
}
