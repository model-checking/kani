// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functions related to codegenning MIR static variables into gotoc

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Symbol;
use rustc_hir::def_id::DefId;
use rustc_middle::mir::mono::MonoItem;
use rustc_middle::ty::{subst::InternalSubsts, Instance};
use tracing::debug;

/// Separator used to generate function static variable names (<function_name>::<variable_name>).
const SEPARATOR: &str = "::";

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_static(&mut self, def_id: DefId, item: MonoItem<'tcx>) {
        debug!("codegen_static");
        let alloc = self.tcx.eval_static_initializer(def_id).unwrap();
        let symbol_name = item.symbol_name(self.tcx).to_string();
        self.codegen_allocation(alloc.inner(), |_| symbol_name.clone(), Some(symbol_name.clone()));
    }

    pub fn declare_static(&mut self, def_id: DefId, item: MonoItem<'tcx>) {
        // Unique mangled monomorphized name.
        let symbol_name = item.symbol_name(self.tcx).to_string();
        // Pretty name which may include function name.
        let pretty_name = Instance::new(def_id, InternalSubsts::empty()).to_string();
        // Name of the variable in the local context.
        let base_name =
            pretty_name.rsplit_once(SEPARATOR).map(|names| names.1).unwrap_or(pretty_name.as_str());
        debug!(?symbol_name, ?pretty_name, ?base_name, "declare_static {}", item);

        let typ = self.codegen_ty(self.tcx.type_of(def_id));
        let span = self.tcx.def_span(def_id);
        let location = self.codegen_span(&span);
        let symbol = Symbol::static_variable(symbol_name, base_name, typ, location)
            .with_is_hidden(false) // Static items are always user defined.
            .with_pretty_name(pretty_name);
        self.symbol_table.insert(symbol);
    }
}
