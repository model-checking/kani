// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `volatile_load` returns the value pointed to by the pointer
// passed as the argument.
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let vec = vec![1, 2];
    let vec_ptr = vec.as_ptr();
    let fst_val = unsafe { std::intrinsics::volatile_load(vec_ptr) };
    assert_eq!(fst_val, 1);
    let snd_val = unsafe { std::intrinsics::volatile_load(vec_ptr.add(1)) };
    assert_eq!(snd_val, 2);
}
