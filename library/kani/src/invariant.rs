// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the Invariant trait.

/// This trait should be used to specify and check type safety invariants for a
/// type.
pub trait Invariant
where
    Self: Sized,
{
    fn is_valid(&self) -> bool;
}
