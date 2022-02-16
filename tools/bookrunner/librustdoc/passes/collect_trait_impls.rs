//! Collects trait impls for each item in the crate. For example, if a crate
//! defines a struct that implements a trait, this pass will note that the
//! struct implements that trait.
use super::Pass;
use crate::clean::*;
use crate::core::DocContext;
use crate::visit::DocVisitor;

use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::DefIdTree;
use rustc_span::symbol::sym;

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

impl ItemCollector {
    fn new() -> Self {
        Self::default()
    }
}

impl DocVisitor for ItemCollector {
    fn visit_item(&mut self, i: &Item) {
        self.items.insert(i.def_id);

        self.visit_item_recur(i)
    }
}

struct BadImplStripper {
    prims: FxHashSet<PrimitiveType>,
    items: FxHashSet<ItemId>,
}

impl BadImplStripper {
    fn keep_impl(&self, ty: &Type, is_deref: bool) -> bool {
        if let Generic(_) = ty {
            // keep impls made on generics
            true
        } else if let Some(prim) = ty.primitive_type() {
            self.prims.contains(&prim)
        } else if let Some(did) = ty.def_id_no_primitives() {
            is_deref || self.keep_impl_with_def_id(did.into())
        } else {
            false
        }
    }

    fn keep_impl_with_def_id(&self, did: ItemId) -> bool {
        self.items.contains(&did)
    }
}
