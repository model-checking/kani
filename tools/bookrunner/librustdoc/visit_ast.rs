// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! The Rust AST Visitor. Extracts useful information and massages it into a form
//! usable for `clean`.

use rustc_hir as hir;
use rustc_middle::ty::TyCtxt;
use rustc_span::symbol::{sym, Symbol};
use rustc_span::Span;

use crate::clean::{AttributesExt, NestedAttributesExt};

/// This module is used to store stuff from Rust's AST in a more convenient
/// manner (and with prettier names) before cleaning.
#[derive(Debug)]
pub(crate) struct Module<'hir> {
    pub(crate) name: Symbol,
    pub(crate) where_inner: Span,
    pub(crate) mods: Vec<Module<'hir>>,
    pub(crate) id: hir::HirId,
    // (item, renamed)
    pub(crate) items: Vec<(&'hir hir::Item<'hir>, Option<Symbol>)>,
    pub(crate) foreigns: Vec<(&'hir hir::ForeignItem<'hir>, Option<Symbol>)>,
}

impl Module<'_> {
    pub(crate) fn where_outer(&self, tcx: TyCtxt<'_>) -> Span {
        tcx.hir().span(self.id)
    }
}

pub(crate) fn inherits_doc_hidden(tcx: TyCtxt<'_>, mut node: hir::HirId) -> bool {
    while let Some(id) = tcx.hir().get_enclosing_scope(node) {
        node = id;
        if tcx.hir().attrs(node).lists(sym::doc).has_word(sym::hidden) {
            return true;
        }
    }
    false
}
