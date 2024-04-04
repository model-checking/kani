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
use rustc_data_structures::fingerprint::Fingerprint;
use rustc_hir::{def_id::DefId, definitions::DefPathHash};
use rustc_index::IndexVec;
use rustc_middle::mir::{
    visit::MutVisitor, Body, Const, ConstValue, Local, LocalDecl, Location, Operand,
};
use rustc_middle::ty::{self, TyCtxt};

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

/// Returns the new body of a function/method if it has been stubbed out;
/// otherwise, returns the old body.
pub fn transform<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, old_body: &'tcx Body<'tcx>) -> Body<'tcx> {
    if let Some(replacement) = get_stub(tcx, def_id) {
        debug!(
            original = tcx.def_path_debug_str(def_id),
            replaced = tcx.def_path_debug_str(replacement),
            "transform"
        );
        let new_body = tcx.optimized_mir(replacement).clone();
        if check_compatibility(tcx, def_id, old_body, replacement, &new_body) {
            return new_body;
        }
    }
    old_body.clone()
}

/// Traverse `body` searching for calls to foreing functions and, whevever there is
/// a stub available, replace the call to the foreign function with a call
/// to its correspondent stub. This happens as a separate step because there is no
/// body available to foreign functions at this stage.
pub fn transform_foreign_functions<'tcx>(tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
    if let Some(stub_map) = get_stub_mapping(tcx) {
        let mut visitor =
            ForeignFunctionTransformer { tcx, local_decls: body.clone().local_decls, stub_map };
        visitor.visit_body(body);
    }
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

struct ForeignFunctionTransformer<'tcx> {
    /// The compiler context.
    tcx: TyCtxt<'tcx>,
    /// Local declarations of the callee function. Kani searches here for foreign functions.
    local_decls: IndexVec<Local, LocalDecl<'tcx>>,
    /// Map of functions/methods to their correspondent stubs.
    stub_map: HashMap<DefId, DefId>,
}

impl<'tcx> MutVisitor<'tcx> for ForeignFunctionTransformer<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_operand(&mut self, operand: &mut Operand<'tcx>, _location: Location) {
        let func_ty = operand.ty(&self.local_decls, self.tcx);
        if let ty::FnDef(reachable_function, arguments) = *func_ty.kind() {
            if self.tcx.is_foreign_item(reachable_function) {
                if let Some(stub) = self.stub_map.get(&reachable_function) {
                    let Operand::Constant(function_definition) = operand else {
                        return;
                    };
                    function_definition.const_ = Const::from_value(
                        ConstValue::ZeroSized,
                        self.tcx.type_of(stub).instantiate(self.tcx, arguments),
                    );
                }
            }
        }
    }
}

/// Checks whether the stub is compatible with the original function/method: do
/// the arities and types (of the parameters and return values) match up? This
/// does **NOT** check whether the type variables are constrained to implement
/// the same traits; trait mismatches are checked during monomorphization.
fn check_compatibility<'a, 'tcx>(
    tcx: TyCtxt,
    old_def_id: DefId,
    old_body: &'a Body<'tcx>,
    stub_def_id: DefId,
    stub_body: &'a Body<'tcx>,
) -> bool {
    // Check whether the arities match.
    if old_body.arg_count != stub_body.arg_count {
        tcx.dcx().span_err(
            tcx.def_span(stub_def_id),
            format!(
                "arity mismatch: original function/method `{}` takes {} argument(s), stub `{}` takes {}",
                tcx.def_path_str(old_def_id),
                old_body.arg_count,
                tcx.def_path_str(stub_def_id),
                stub_body.arg_count
            ),
        );
        return false;
    }
    // Check whether the numbers of generic parameters match.
    let old_num_generics = tcx.generics_of(old_def_id).count();
    let stub_num_generics = tcx.generics_of(stub_def_id).count();
    if old_num_generics != stub_num_generics {
        tcx.dcx().span_err(
            tcx.def_span(stub_def_id),
            format!(
                "mismatch in the number of generic parameters: original function/method `{}` takes {} generic parameters(s), stub `{}` takes {}",
                tcx.def_path_str(old_def_id),
                old_num_generics,
                tcx.def_path_str(stub_def_id),
                stub_num_generics
            ),
        );
        return false;
    }
    // Check whether the types match. Index 0 refers to the returned value,
    // indices [1, `arg_count`] refer to the parameters.
    // TODO: We currently force generic parameters in the stub to have exactly
    // the same names as their counterparts in the original function/method;
    // instead, we should be checking for the equivalence of types up to the
    // renaming of generic parameters.
    // <https://github.com/model-checking/kani/issues/1953>
    let mut matches = true;
    for i in 0..=old_body.arg_count {
        let old_arg = old_body.local_decls.get(i.into()).unwrap();
        let new_arg = stub_body.local_decls.get(i.into()).unwrap();
        if old_arg.ty != new_arg.ty {
            let prefix = if i == 0 {
                "return type differs".to_string()
            } else {
                format!("type of parameter {} differs", i - 1)
            };
            tcx.dcx().span_err(
                new_arg.source_info.span,
                format!(
                    "{prefix}: stub `{}` has type `{}` where original function/method `{}` has type `{}`",
                    tcx.def_path_str(stub_def_id),
                    new_arg.ty,
                    tcx.def_path_str(old_def_id),
                    old_arg.ty
                ),
            );
            matches = false;
        }
    }
    matches
}

/// The prefix we will use when serializing the stub mapping as a rustc argument.
const RUSTC_ARG_PREFIX: &str = "kani_stubs=";

/// Serializes the stub mapping into a rustc argument.
pub fn mk_rustc_arg(stub_mapping: &BTreeMap<DefPathHash, DefPathHash>) -> String {
    // Serialize each `DefPathHash` as a pair of `u64`s, and the whole mapping
    // as an association list.
    let mut pairs = Vec::new();
    for (k, v) in stub_mapping {
        let (k_a, k_b) = k.0.split();
        let kparts = (k_a.as_u64(), k_b.as_u64());
        let (v_a, v_b) = v.0.split();
        let vparts = (v_a.as_u64(), v_b.as_u64());
        pairs.push((kparts, vparts));
    }
    // Store our serialized mapping as a fake LLVM argument (safe to do since
    // LLVM will never see them).
    format!("-Cllvm-args='{RUSTC_ARG_PREFIX}{}'", serde_json::to_string(&pairs).unwrap())
}

/// Deserializes the stub mapping from the rustc argument value.
fn deserialize_mapping(tcx: TyCtxt, val: &str) -> HashMap<DefId, DefId> {
    type Item = (u64, u64);
    let item_to_def_id = |item: Item| -> DefId {
        let hash = DefPathHash(Fingerprint::new(item.0, item.1));
        tcx.def_path_hash_to_def_id(hash, &mut || panic!())
    };
    let pairs: Vec<(Item, Item)> = serde_json::from_str(val).unwrap();
    let mut m = HashMap::default();
    for (k, v) in pairs {
        let kid = item_to_def_id(k);
        let vid = item_to_def_id(v);
        m.insert(kid, vid);
    }
    m
}

/// Retrieves the stub mapping from the compiler configuration.
fn get_stub_mapping(tcx: TyCtxt) -> Option<HashMap<DefId, DefId>> {
    // Use a static so that we compile the regex only once.
    lazy_static! {
        static ref RE: Regex = Regex::new(&format!("'{RUSTC_ARG_PREFIX}(.*)'")).unwrap();
    }
    for arg in &tcx.sess.opts.cg.llvm_args {
        if let Some(captures) = RE.captures(arg) {
            return Some(deserialize_mapping(tcx, captures.get(1).unwrap().as_str()));
        }
    }
    None
}
