// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem;

fn main() {
    let mut var1 = kani::any::<i32>();
    let mut var2 = kani::any::<i32>();
    let old_var1 = var1;
    let old_var2 = var2;
    unsafe {
        mem::swap(&mut var1, &mut var2);
    }
    assert_eq!(var1, old_var2);
    assert_eq!(var2, old_var1);
}
