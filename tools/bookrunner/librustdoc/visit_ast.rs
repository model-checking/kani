//! The Rust AST Visitor. Extracts useful information and massages it into a form
//! usable for `clean`.

use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::Node;
use rustc_hir::CRATE_HIR_ID;
use rustc_middle::middle::privacy::AccessLevel;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{CRATE_DEF_ID, LOCAL_CRATE};
use rustc_span::symbol::{kw, sym, Symbol};
use rustc_span::Span;

use std::mem;

use crate::clean::{self, cfg::Cfg, AttributesExt, NestedAttributesExt};
use crate::core;

/// This module is used to store stuff from Rust's AST in a more convenient
/// manner (and with prettier names) before cleaning.
#[derive(Debug)]
crate struct Module<'hir> {
    crate name: Symbol,
    crate where_inner: Span,
    crate mods: Vec<Module<'hir>>,
    crate id: hir::HirId,
    // (item, renamed)
    crate items: Vec<(&'hir hir::Item<'hir>, Option<Symbol>)>,
    crate foreigns: Vec<(&'hir hir::ForeignItem<'hir>, Option<Symbol>)>,
}

impl Module<'_> {
    crate fn new(name: Symbol, id: hir::HirId, where_inner: Span) -> Self {
        Module { name, id, where_inner, mods: Vec::new(), items: Vec::new(), foreigns: Vec::new() }
    }

    crate fn where_outer(&self, tcx: TyCtxt<'_>) -> Span {
        tcx.hir().span(self.id)
    }
}

crate fn inherits_doc_hidden(tcx: TyCtxt<'_>, mut node: hir::HirId) -> bool {
    while let Some(id) = tcx.hir().get_enclosing_scope(node) {
        node = id;
        if tcx.hir().attrs(node).lists(sym::doc).has_word(sym::hidden) {
            return true;
        }
    }
    false
}
