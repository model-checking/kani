// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks pointers to a function item. A function item is zero-sized, but a pointer to one is an
//! ordinary value that holds the address it was given.
//! See <https://github.com/model-checking/kani/issues/2255>.

fn foo() -> u32 {
    42
}

fn null_of<T>(_: T) -> *const T {
    core::ptr::null()
}

/// The program from the issue: `{:p}` formats the address of the function item.
#[kani::proof]
#[allow(function_item_references)]
fn check_print_address() {
    println!("{:p}", &foo);
}

#[kani::proof]
fn check_pointer_value() {
    let p: *const _ = &foo;
    assert!(!p.is_null());
    assert!(null_of(foo).is_null());
}

/// Fails if a pointer to a function item is read as some other address than the one assigned.
#[kani::proof]
#[kani::should_panic]
fn check_null_stays_null() {
    assert!(!null_of(foo).is_null());
}
