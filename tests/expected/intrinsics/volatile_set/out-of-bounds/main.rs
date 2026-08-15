// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `volatile_set_memory` fails if an out-of-bounds write is made.
// Mirrors the equivalent `write_bytes` test, since the two share codegen: this
// pins that reusing `codegen_write_bytes` really does carry its safety checks
// over to the volatile variant, rather than only its happy path.
#![feature(core_intrinsics)]
use std::intrinsics::volatile_set_memory;

#[kani::proof]
fn main() {
    let mut vec = vec![0u32; 4];
    unsafe {
        let vec_ptr = vec.as_mut_ptr().add(4);
        volatile_set_memory(vec_ptr, 0xfe, 1);
    }
    assert_eq!(vec, [0, 0, 0, 0]);
}
