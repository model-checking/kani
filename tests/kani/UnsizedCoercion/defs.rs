// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --only-codegen
//! Definitions of traits and structs to be used in the unsized coercion tests.
//! We skip running the verification since there's no harness in this file.

use std::ops::Deref;

/// Trait that returns an ID.
pub trait Identity {
    fn id(&self) -> u16;
}

/// Outer struct which wraps another Identity type.
pub struct Outer<T: ?Sized> {
    pub outer_id: u8,
    pub inner: T,
}

/// Inner type that implements Identity.
pub struct Inner {
    pub id: u8,
}

// Implementation for cases where T implements Identity.
impl<T> Identity for Outer<T>
where
    T: ?Sized + Identity,
{
    fn id(&self) -> u16 {
        ((self.outer_id as u16) << 8) + (self.inner.id() as u16)
    }
}

impl Identity for Inner {
    fn id(&self) -> u16 {
        self.id.into()
    }
}

/// Get the id from a fat pointer.
#[allow(dead_code)]
pub fn id_from_dyn(identity: &dyn Identity) -> u16 {
    identity.id()
}

/// Get the id from a smart pointer.
#[allow(dead_code)]
pub fn id_from_coerce<T>(identity: T) -> u16
where
    T: Deref<Target = dyn Identity>,
{
    identity.id()
}
