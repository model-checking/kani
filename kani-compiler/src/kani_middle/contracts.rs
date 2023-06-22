// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Basic type definitions for function contracts.
use rustc_hir::def_id::DefId;

/// Generic representation for a function contract. This is so that we can reuse
/// this type for different resolution stages if the implementation functions
/// (`C`).
#[derive(Default)]
pub struct GFnContract<C> {
    requires: Vec<C>,
    ensures: Vec<C>,
    assigns: Vec<C>,
}

pub type FnContract = GFnContract<DefId>;

impl<C> GFnContract<C> {
    /// Read access to all preondition clauses.
    pub fn requires(&self) -> &[C] {
        &self.requires
    }

    /// Read access to all postcondition clauses.
    pub fn ensures(&self) -> &[C] {
        &self.ensures
    }

    pub fn new(requires: Vec<C>, ensures: Vec<C>, assigns: Vec<C>) -> Self {
        Self { requires, ensures, assigns }
    }

    /// Perform a transformation on each implementation item. Usually these are
    /// resolution steps.
    pub fn map<C0, F: FnMut(&C) -> C0>(&self, mut f: F) -> GFnContract<C0> {
        GFnContract {
            requires: self.requires.iter().map(&mut f).collect(),
            ensures: self.ensures.iter().map(&mut f).collect(),
            assigns: self.assigns.iter().map(&mut f).collect(),
        }
    }

    /// If this is false, then this contract has no clauses and can safely be ignored.
    pub fn enforceable(&self) -> bool {
        !self.requires().is_empty() || !self.ensures().is_empty()
    }
}
