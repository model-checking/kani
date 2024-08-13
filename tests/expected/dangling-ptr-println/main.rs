// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! These tests check that Kani correctly detects dangling pointer dereference inside println macro.
//! Related issue: <https://github.com/model-checking/kani/issues/3235>.

fn reference_dies() -> *mut i32 {
    let mut x: i32 = 2;
    &mut x as *mut i32
}

#[kani::proof]
fn local_unsafe() {
    let x = reference_dies();
    println!("My pointer, {}", unsafe { *x });
}

#[kani::proof]
unsafe fn general_unsafe() {
    let x = reference_dies();
    println!("My pointer, {}", *x);
}

#[kani::proof]
fn unsafe_block() {
    let x = reference_dies();
    unsafe {
        println!("{}", *x);
    }
}
