// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functions related to codegenning MIR static variables into gotoc

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Symbol;
use stable_mir::mir::mono::{Instance, StaticDef};
use stable_mir::CrateDef;
use tracing::debug;

impl<'tcx> GotocCtx<'tcx> {
    /// Ensures a static variable is initialized.
    ///
    /// Note that each static variable have their own location in memory. Per Rust documentation:
    /// "statics declare global variables. These represent a memory address."
    /// Source: <https://rust-lang.github.io/rfcs/0246-const-vs-static.html>
    pub fn codegen_static(&mut self, def: StaticDef) {
        debug!("codegen_static");
        let alloc = def.eval_initializer().unwrap();
        let symbol_name = Instance::from(def).mangled_name();
        self.codegen_alloc_in_memory(alloc, symbol_name);
    }

    /// Mutates the Goto-C symbol table to add a forward-declaration of the static variable.
    pub fn declare_static(&mut self, def: StaticDef) {
        let instance = Instance::from(def);
        // Unique mangled monomorphized name.
        let symbol_name = instance.mangled_name();
        // Pretty name which may include function name.
        let pretty_name = instance.name();
        debug!(?def, ?symbol_name, ?pretty_name, "declare_static");

        let typ = self.codegen_ty_stable(instance.ty());
        let location = self.codegen_span_stable(def.span());
        let symbol = Symbol::static_variable(symbol_name.clone(), symbol_name, typ, location)
            .with_is_hidden(false) // Static items are always user defined.
            .with_pretty_name(pretty_name);
        self.symbol_table.insert(symbol);
    }
}
