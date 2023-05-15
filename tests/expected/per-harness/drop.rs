// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness check_drop_foo

//! Test that Kani reachability analysis remove other unrelated drop implementations when using
//! fat pointers.
//! For each harness, there should be 1 of 1 cover satisfied.

use std::any::Any;
use std::fmt::Debug;
use std::ptr::drop_in_place;

#[derive(Debug)]
struct Foo {}

#[derive(Debug)]
struct Bar {}

impl Drop for Foo {
    fn drop(&mut self) {
        kani::cover!(true, "DropFoo");
    }
}

impl Drop for Bar {
    fn drop(&mut self) {
        // This cover should be excluded from the result since there is no CFG path that connects
        // the harness `check_drop_foo` with this function call.
        kani::cover!(true, "DropBar");
    }
}

#[kani::proof]
fn check_drop_foo() {
    let boxed: Box<dyn Debug> = Box::new(Foo {});
    unsafe { drop_in_place(Box::into_raw(boxed)) };
}

#[kani::proof]
fn check_drop_bar() {
    let boxed: Box<dyn Any> = Box::new(Bar {});
    unsafe { drop_in_place(Box::into_raw(boxed)) };
}

#[kani::proof]
fn check_drop_bar_debug() {
    let boxed: Box<dyn Debug> = Box::new(Bar {});
    unsafe { drop_in_place(Box::into_raw(boxed)) };
}
