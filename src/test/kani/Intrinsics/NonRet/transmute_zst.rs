// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#![feature(never_type)]

// Transmutes an inhabited ZST into a uninhabited ZST
//
// Handled as a special case of transmute (non returning intrinsic) that
// compiles but crashes at runtime, similar to calling `std::intrinsic::abort`
fn main() {
    unsafe { std::mem::transmute::<(), !>(()) };
}
