// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `transmute` works as expected when turning a pointer into
// a function pointer.

// This test is a modified version of the example found in
// https://doc.rust-lang.org/std/intrinsics/fn.transmute.html

fn foo() -> i32 {
    0
}

#[kani::proof]
fn main() {
    let pointer = foo as *const ();
    let function = unsafe { std::mem::transmute::<*const (), fn() -> i32>(pointer) };
    assert_eq!(function(), 0);
}
