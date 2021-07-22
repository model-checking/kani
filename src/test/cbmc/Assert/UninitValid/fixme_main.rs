// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

use std::mem::{self, MaybeUninit};

fn main() {
    // The compiler assumes that variables are properly initialized according to
    // the requirements of the variable's type (e.g., a variable of reference
    // type must be aligned and non-NULL). This is an invariant that must always
    // be upheld - even in unsafe code.
    // https://doc.rust-lang.org/std/mem/union.MaybeUninit.html

    let _x1: &i32 = unsafe { mem::uninitialized() }; // undefined behavior!

    // The compiler warns that std::mem::uninitialized() is deprecated
    // and mem::MaybeUninit should be used instead
    let _x2: &i32 = unsafe { MaybeUninit::uninit().assume_init() }; // undefined behavior!
}
