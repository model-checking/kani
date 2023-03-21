// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that Kani works when the target directory is a symlink,
//! which wasn't working previously (see
//! https://github.com/model-checking/kani/issues/2303)

#[kani::proof]
fn main() {
    let x: i32 = kani::any();
    let y: i32 = kani::any();
    kani::assume(y == 0);
    assert_eq!(x + y, x);
}
