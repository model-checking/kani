// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code related to the MIR-to-MIR pass that performs the
//! stubbing of functions and methods. The primary function of the module is
//! `transform`, which takes the `DefId` of a function/method and returns the
//! body of its stub, if appropriate. The stub mapping it uses is set via rustc
//! arguments.

use std::collections::{BTreeMap, HashMap};

use lazy_static::lazy_static;
use regex::Regex;
use rustc_hir::def_id::DefId;
use rustc_index::IndexVec;
use rustc_middle::mir::{
    visit::MutVisitor, Body, Const, ConstValue, Local, LocalDecl, Location, Operand,
};
use rustc_middle::ty::{self, TyCtxt};

use crate::kani_middle::stubbing::check_compatibility;
use tracing::debug;

/// Returns the `DefId` of the stub for the function/method identified by the
/// parameter `def_id`, and `None` if the function/method is not stubbed.
pub fn get_stub(tcx: TyCtxt, def_id: DefId) -> Option<DefId> {
    let stub_map = get_stub_mapping(tcx)?;
    stub_map.get(&def_id).copied()
}

pub fn get_stub_key(tcx: TyCtxt, def_id: DefId) -> Option<DefId> {
    let stub_map = get_stub_mapping(tcx)?;
    stub_map.iter().find_map(|(&key, &val)| if val == def_id { Some(key) } else { None })
}

/// Traverse `body` searching for calls to `kani::any_modifies` and replace these calls
/// with calls to `kani::any`. This happens as a separate step as it is only necessary
/// for contract-generated functions.
pub fn transform_any_modifies<'tcx>(tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
    let mut visitor = AnyModifiesTransformer { tcx, local_decls: body.clone().local_decls };
    visitor.visit_body(body);
}

struct AnyModifiesTransformer<'tcx> {
    /// The compiler context.
    tcx: TyCtxt<'tcx>,
    /// Local declarations of the callee function. Kani searches here for foreign functions.
    local_decls: IndexVec<Local, LocalDecl<'tcx>>,
}

impl<'tcx> MutVisitor<'tcx> for AnyModifiesTransformer<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_operand(&mut self, operand: &mut Operand<'tcx>, _location: Location) {
        let func_ty = operand.ty(&self.local_decls, self.tcx);
        if let ty::FnDef(reachable_function, arguments) = *func_ty.kind() {
            if let Some(any_modifies) = self.tcx.get_diagnostic_name(reachable_function)
                && any_modifies.as_str() == "KaniAnyModifies"
            {
                let Operand::Constant(function_definition) = operand else {
                    return;
                };
                let kani_any_symbol = self
                    .tcx
                    .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniAny"))
                    .expect("We should have a `kani::any()` definition at this point.");
                function_definition.const_ = Const::from_value(
                    ConstValue::ZeroSized,
                    self.tcx.type_of(kani_any_symbol).instantiate(self.tcx, arguments),
                );
            }
        }
    }
}

/// Retrieves the stub mapping from the compiler configuration.
fn get_stub_mapping(_tcx: TyCtxt) -> Option<HashMap<DefId, DefId>> {
    None
}
