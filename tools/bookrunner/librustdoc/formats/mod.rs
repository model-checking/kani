// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
pub(crate) mod cache;
pub(crate) mod item_type;

use rustc_hir::def_id::DefId;

use crate::clean;

/// Metadata about implementations for a type or trait.
#[derive(Clone, Debug)]
pub(crate) struct Impl {
    pub(crate) impl_item: clean::Item,
}

impl Impl {
    pub(crate) fn inner_impl(&self) -> &clean::Impl {
        match *self.impl_item.kind {
            clean::ImplItem(ref impl_) => impl_,
            _ => panic!("non-impl item found in impl"),
        }
    }

    pub(crate) fn trait_did(&self) -> Option<DefId> {
        self.inner_impl().trait_.as_ref().map(|t| t.def_id())
    }
}
