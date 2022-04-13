// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-codegen-fail

// Checks that `forget` produces a compilation error if the value is referenced
// after "forgetting" it

// This test is a modified version of the code found in
// https://doc.rust-lang.org/std/mem/fn.forget.html#relationship-with-manuallydrop
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let mut v = vec![65, 122];
    // Build a `String` using the contents of `v`
    let s = unsafe { String::from_raw_parts(v.as_mut_ptr(), v.len(), v.capacity()) };
    // leak `v` because its memory is now managed by `s`
    std::intrinsics::forget(v); // v is now invalid and must not be passed to a function
    assert!(v[0] == 65); // Error: v is referenced after `forget`
    assert_eq!(s, "Az");
    // `s` is implicitly dropped and its memory deallocated.
}
