// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functions related to codegenning MIR static variables into gotoc

use crate::GotocCtx;
use cbmc::goto_program::Symbol;
use rustc_hir::def_id::DefId;
use rustc_middle::mir::mono::MonoItem;
use tracing::debug;

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_static(&mut self, def_id: DefId, item: MonoItem<'tcx>) {
        debug!("codegen_static");
        let alloc = self.tcx.eval_static_initializer(def_id).unwrap();
        let symbol_name = item.symbol_name(self.tcx).to_string();
        self.codegen_allocation(alloc, |_| symbol_name.clone(), Some(symbol_name.clone()));
    }

    pub fn declare_static(&mut self, def_id: DefId, item: MonoItem<'tcx>) {
        debug!("declare_static {:?}", def_id);
        let symbol_name = item.symbol_name(self.tcx).to_string();
        let typ = self.codegen_ty(self.tcx.type_of(def_id));
        let span = self.tcx.def_span(def_id);
        let location = self.codegen_span(&span);
        let symbol = Symbol::static_variable(symbol_name.to_string(), symbol_name, typ, location);
        self.symbol_table.insert(symbol);
    }
}
