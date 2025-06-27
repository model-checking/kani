// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check the basic coercion when using a custom CoerceUnsized implementation.
//! Tests are broken down into different crates to ensure that the reachability works for each case.
#![feature(coerce_unsized)]
#![feature(unsize)]
use std::marker::Unsize;
use std::ops::{CoerceUnsized, Deref};

mod defs;
use defs::*;

/// Dummy reference wrapper that allow unsized coercion.
pub struct MyPtr<'a, T: ?Sized> {
    ptr: &'a T,
}

/// Implement `CoerceUnsized` type which allow us to convert MyPtr<Struct> to MyPtr<dyn Trait>.
/// <https://doc.rust-lang.org/std/ops/trait.CoerceUnsized.html>
impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<MyPtr<'a, U>> for MyPtr<'a, T> {}

impl<'a, T: ?Sized> Deref for MyPtr<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

#[kani::proof]
fn check_outer_coercion() {
    let inner_id = kani::any();
    let outer_id = kani::any();
    let outer = Outer { inner: Inner { id: inner_id }, outer_id };
    let outer_ptr = MyPtr { ptr: &outer };
    let id_ptr: MyPtr<dyn Identity> = outer_ptr;
    assert_eq!(id_from_coerce(id_ptr) >> 8, outer_id.into());
}
