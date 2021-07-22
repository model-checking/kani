// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-check-fail

#[allow(unconditional_recursion)]
pub unsafe fn drop_in_place<T: ?Sized>(to_drop: *mut T) {
    // Code here does not matter - this is replaced by the
    // real drop glue by the compiler.

    // SAFETY: see comment above
    unsafe { drop_in_place(to_drop) }
}

pub fn main() {
    let mut x = 3;
    drop_in_place(x);
}
