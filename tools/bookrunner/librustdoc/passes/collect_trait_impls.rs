// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! Collects trait impls for each item in the crate. For example, if a crate
//! defines a struct that implements a trait, this pass will note that the
//! struct implements that trait.
use crate::clean::*;
use crate::core::DocContext;
use crate::visit::DocVisitor;

use rustc_data_structures::fx::FxHashSet;

struct SyntheticImplCollector<'a, 'tcx> {
    cx: &'a mut DocContext<'tcx>,
    impls: Vec<Item>,
}

impl<'a, 'tcx> DocVisitor for SyntheticImplCollector<'a, 'tcx> {
    fn visit_item(&mut self, i: &Item) {
        if i.is_struct() || i.is_enum() || i.is_union() {
            // FIXME(eddyb) is this `doc(hidden)` check needed?
            if !self.cx.tcx.is_doc_hidden(i.def_id.expect_def_id()) {
                self.impls
                    .extend(get_auto_trait_and_blanket_impls(self.cx, i.def_id.expect_def_id()));
            }
        }

        self.visit_item_recur(i)
    }
}

#[derive(Default)]
struct ItemCollector {
    items: FxHashSet<ItemId>,
}

impl ItemCollector {}

impl DocVisitor for ItemCollector {
    fn visit_item(&mut self, i: &Item) {
        self.items.insert(i.def_id);

        self.visit_item_recur(i)
    }
}
