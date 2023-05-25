// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functions related to codegenning MIR static variables into gotoc

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Symbol;
use rustc_hir::def_id::DefId;
use rustc_middle::mir::mono::MonoItem;
use rustc_middle::ty::{subst::InternalSubsts, Instance};
use tracing::debug;

impl<'tcx> GotocCtx<'tcx> {
    /// Ensures a static variable is initialized.
    ///
    /// Note that each static variable have their own location in memory. Per Rust documentation:
    /// "statics declare global variables. These represent a memory address."
    /// Source: <https://rust-lang.github.io/rfcs/0246-const-vs-static.html>
    pub fn codegen_static(&mut self, def_id: DefId, item: MonoItem<'tcx>) {
        debug!("codegen_static");
        let alloc = self.tcx.eval_static_initializer(def_id).unwrap();
        let symbol_name = item.symbol_name(self.tcx).to_string();
        self.codegen_alloc_in_memory(alloc.inner(), symbol_name);
    }

    /// Mutates the Goto-C symbol table to add a forward-declaration of the static variable.
    pub fn declare_static(&mut self, def_id: DefId, item: MonoItem<'tcx>) {
        // Unique mangled monomorphized name.
        let symbol_name = item.symbol_name(self.tcx).to_string();
        // Pretty name which may include function name.
        let pretty_name = Instance::new(def_id, InternalSubsts::empty()).to_string();
        debug!(?symbol_name, ?pretty_name, "declare_static {}", item);

        let typ = self.codegen_ty(self.tcx.type_of(def_id).subst_identity());
        let span = self.tcx.def_span(def_id);
        let location = self.codegen_span(&span);
        let symbol = Symbol::static_variable(symbol_name.clone(), symbol_name, typ, location)
            .with_is_hidden(false) // Static items are always user defined.
            .with_pretty_name(pretty_name);
        self.symbol_table.insert(symbol);
    }
}
