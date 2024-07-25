// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code related to the MIR-to-MIR pass to enable contracts.
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::find_fn_def;
use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use cbmc::{InternString, InternedString};
use rustc_hir::def_id::DefId as InternalDefId;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use rustc_span::Symbol;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    Body, ConstOperand, Operand, Rvalue, Terminator, TerminatorKind, VarDebugInfoContents,
};
use stable_mir::ty::{ClosureDef, FnDef, MirConst, RigidTy, TyKind, TypeAndMut, UintTy};
use stable_mir::CrateDef;
use std::collections::HashSet;
use std::fmt::Debug;
use tracing::{debug, trace};

/// Check if we can replace calls to any_modifies or write_any.
///
/// This pass will replace the entire body, and it should only be applied to stubs
/// that have a body.
///
/// write_any is replaced with one of write_any_slim, write_any_slice, or write_any_str
/// depending on what the type of the input it
///
/// any_modifies is replaced with any
#[derive(Debug)]
pub struct AnyModifiesPass {
    kani_any: Option<FnDef>,
    kani_any_modifies: Option<FnDef>,
    kani_write_any: Option<FnDef>,
    kani_write_any_slim: Option<FnDef>,
    kani_write_any_slice: Option<FnDef>,
    kani_write_any_str: Option<FnDef>,
    target_fn: Option<InternedString>,
}

impl TransformPass for AnyModifiesPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        // TODO: Check if the harness has proof_for_contract
        query_db.args().unstable_features.contains(&"function-contracts".to_string())
            && self.kani_any.is_some()
    }

    /// Transform the function body by replacing it with the stub body.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "AnyModifiesPass::transform");

        if instance.def.def_id() == self.kani_any.unwrap().def_id() {
            // Ensure kani::any is valid.
            self.any_body(tcx, body)
        } else if instance.ty().kind().is_closure() {
            // Replace any modifies occurrences. They should only happen in the contract closures.
            self.replace_any_modifies(body)
        } else {
            (false, body)
        }
    }
}

impl AnyModifiesPass {
    /// Build the pass with non-extern function stubs.
    pub fn new(tcx: TyCtxt, unit: &CodegenUnit) -> AnyModifiesPass {
        let item_fn_def = |item| {
            let TyKind::RigidTy(RigidTy::FnDef(def, _)) =
                rustc_internal::stable(tcx.type_of(item)).value.kind()
            else {
                unreachable!("Expected function, but found `{:?}`", tcx.def_path_str(item))
            };
            def
        };
        let kani_any =
            tcx.get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniAny")).map(item_fn_def);
        let kani_any_modifies = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniAnyModifies"))
            .map(item_fn_def);
        let kani_write_any = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniWriteAny"))
            .map(item_fn_def);
        let kani_write_any_slim = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniWriteAnySlim"))
            .map(item_fn_def);
        let kani_write_any_slice = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniWriteAnySlice"))
            .map(item_fn_def);
        let kani_write_any_str = tcx
            .get_diagnostic_item(rustc_span::symbol::Symbol::intern("KaniWriteAnyStr"))
            .map(item_fn_def);
        let target_fn = if let Some(harness) = unit.harnesses.first() {
            let attributes = KaniAttributes::for_instance(tcx, *harness);
            let target_fn =
                attributes.proof_for_contract().map(|symbol| symbol.unwrap().as_str().intern());
            target_fn
        } else {
            None
        };
        AnyModifiesPass {
            kani_any,
            kani_any_modifies,
            kani_write_any,
            kani_write_any_slim,
            kani_write_any_slice,
            kani_write_any_str,
            target_fn,
        }
    }

    /// Replace calls to `any_modifies` by calls to `any`.
    fn replace_any_modifies(&self, mut body: Body) -> (bool, Body) {
        let mut changed = false;
        let locals = body.locals().to_vec();
        for bb in body.blocks.iter_mut() {
            let TerminatorKind::Call { func, args, .. } = &mut bb.terminator.kind else {
                continue;
            };
            if let TyKind::RigidTy(RigidTy::FnDef(def, instance_args)) =
                func.ty(&locals).unwrap().kind()
                && Some(def) == self.kani_any_modifies
            {
                let instance = Instance::resolve(self.kani_any.unwrap(), &instance_args).unwrap();
                let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                let span = bb.terminator.span;
                let new_func = ConstOperand { span, user_ty: None, const_: literal };
                *func = Operand::Constant(new_func);
                changed = true;
            }

            // if this is a valid kani::write_any function
            if let TyKind::RigidTy(RigidTy::FnDef(def, instance_args)) =
                func.ty(&locals).unwrap().kind()
                && Some(def) == self.kani_write_any
                && args.len() == 1
                && let Some(fn_sig) = func.ty(&locals).unwrap().kind().fn_sig()
                && let Some(TypeAndMut { ty: internal_type, mutability: _ }) =
                    fn_sig.skip_binder().inputs()[0].kind().builtin_deref(true)
            {
                // case on the type of the input
                if let TyKind::RigidTy(RigidTy::Slice(_)) = internal_type.kind() {
                    //if the input is a slice, use write_any_slice
                    let instance =
                        Instance::resolve(self.kani_write_any_slice.unwrap(), &instance_args)
                            .unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                } else if let TyKind::RigidTy(RigidTy::Str) = internal_type.kind() {
                    //if the input is a str, use write_any_str
                    let instance =
                        Instance::resolve(self.kani_write_any_str.unwrap(), &instance_args)
                            .unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                } else {
                    //otherwise, use write_any_slim
                    let instance =
                        Instance::resolve(self.kani_write_any_slim.unwrap(), &instance_args)
                            .unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                }
                changed = true;
            }
        }
        (changed, body)
    }

    /// Check if T::Arbitrary requirement for `kani::any()` is met after replacement.
    ///
    /// If it T does not implement arbitrary, generate error and delete body to interrupt analysis.
    fn any_body(&self, tcx: TyCtxt, mut body: Body) -> (bool, Body) {
        let mut valid = true;
        let locals = body.locals().to_vec();
        for bb in body.blocks.iter_mut() {
            let TerminatorKind::Call { func, .. } = &mut bb.terminator.kind else {
                continue;
            };
            if let TyKind::RigidTy(RigidTy::FnDef(def, args)) = func.ty(&locals).unwrap().kind() {
                match Instance::resolve(def, &args) {
                    Ok(_) => {}
                    Err(e) => {
                        valid = false;
                        debug!(?e, "AnyModifiesPass::any_body failed");
                        let receiver_ty = args.0[0].expect_ty();
                        let msg = if self.target_fn.is_some() {
                            format!(
                                "`{receiver_ty}` doesn't implement `kani::Arbitrary`.\
                                        Please, check `{}` contract.",
                                self.target_fn.unwrap(),
                            )
                        } else {
                            format!("`{receiver_ty}` doesn't implement `kani::Arbitrary`.")
                        };
                        tcx.dcx()
                            .struct_span_err(rustc_internal::internal(tcx, bb.terminator.span), msg)
                            .with_help(
                                "All objects in the modifies clause must implement the Arbitrary. \
                                 The return type must also implement the Arbitrary trait if you \
                                 are checking recursion or using verified stub.",
                            )
                            .emit();
                    }
                }
            }
        }
        if valid {
            (true, body)
        } else {
            let mut new_body = MutableBody::from(body);
            new_body.clear_body(TerminatorKind::Unreachable);
            (true, new_body.into())
        }
    }
}

/// This pass will transform functions annotated with contracts based on the harness configuration.
///
/// Functions with contract will always follow the same structure:
///
/// ```ignore
/// #[kanitool::recursion_check = "__kani_recursion_check_modify"]
/// #[kanitool::checked_with = "__kani_check_modify"]
/// #[kanitool::replaced_with = "__kani_replace_modify"]
/// #[kanitool::inner_check = "__kani_modifies_modify"]
/// fn name_fn(ptr: &mut u32) {
///     #[kanitool::fn_marker = "kani_register_contract"]
///     pub const fn kani_register_contract<T, F: FnOnce() -> T>(f: F) -> T {
///         kani::panic("internal error: entered unreachable code: ")
///     }
///     let kani_contract_mode = kani::internal::mode();
///     match kani_contract_mode {
///         kani::internal::RECURSION_CHECK => {
///             #[kanitool::is_contract_generated(recursion_check)]
///             let mut __kani_recursion_check_name_fn = || { /* recursion check body */ };
///             kani_register_contract(__kani_recursion_check_modify)
///         }
///         kani::internal::REPLACE => {
///             #[kanitool::is_contract_generated(replace)]
///             let mut __kani_replace_name_fn = || { /* replace body */ };
///             kani_register_contract(__kani_replace_name_fn)
///         }
///         kani::internal::SIMPLE_CHECK => {
///             #[kanitool::is_contract_generated(check)]
///             let mut __kani_check_name_fn = || { /* check body */ };
///             kani_register_contract(__kani_check_name_fn)
///         }
///         _ => { /* original body */ }
///     }
/// }
/// ```
///
/// This pass will perform the following operations:
/// 1. For functions with contract that are not being used for check or replacement:
///    - Set `kani_contract_mode` to the value ORIGINAL.
///    - Replace the generated closures body with unreachable.
/// 2. For functions with contract that are being used:
///    - Set `kani_contract_mode` to the value corresponding to the expected usage.
///    - Replace the non-used generated closures body with unreachable.
/// 3. Replace the body of `kani_register_contract` by `kani::internal::run_contract_fn` to
///    invoke the closure.
#[derive(Debug, Default)]
pub struct FunctionWithContractPass {
    /// Function that is being checked, if any.
    check_fn: Option<InternalDefId>,
    /// Functions that should be stubbed by their contract.
    replace_fns: HashSet<InternalDefId>,
    /// Functions annotated with contract attributes will contain contract closures even if they
    /// are not to be used in this harness.
    /// In order to avoid bringing unnecessary logic, we clear their body.
    unused_closures: HashSet<ClosureDef>,
}

impl TransformPass for FunctionWithContractPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, _query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        true
    }

    /// Transform the function body by replacing it with the stub body.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "FunctionWithContractPass::transform");
        match instance.ty().kind().rigid().unwrap() {
            RigidTy::FnDef(def, args) => {
                if let Some(mode) = self.contract_mode(tcx, *def) {
                    self.mark_unused(tcx, *def, &body, mode);
                    let new_body = self.set_mode(tcx, body, mode);
                    (true, new_body)
                } else if KaniAttributes::for_instance(tcx, instance).fn_marker()
                    == Some(Symbol::intern("kani_register_contract"))
                {
                    let run = Instance::resolve(find_fn_def(tcx, "KaniRunContract").unwrap(), args)
                        .unwrap();
                    (true, run.body().unwrap())
                } else {
                    // Not a contract annotated function
                    (false, body)
                }
            }
            RigidTy::Closure(def, _args) => {
                if self.unused_closures.contains(def) {
                    // Delete body and mark it as unreachable.
                    let mut new_body = MutableBody::from(body);
                    new_body.clear_body(TerminatorKind::Unreachable);
                    (true, new_body.into())
                } else {
                    // Not a contract annotated function
                    (false, body)
                }
            }
            _ => {
                /* static variables case */
                (false, body)
            }
        }
    }
}

impl FunctionWithContractPass {
    /// Build the pass by collecting which functions we are stubbing and which ones we are
    /// verifying.
    pub fn new(tcx: TyCtxt, unit: &CodegenUnit) -> FunctionWithContractPass {
        if let Some(harness) = unit.harnesses.first() {
            let attrs = KaniAttributes::for_instance(tcx, *harness);
            let check_fn = attrs.interpret_for_contract_attribute().map(|(_, def_id, _)| def_id);
            let replace_fns: HashSet<_> = attrs
                .interpret_stub_verified_attribute()
                .iter()
                .map(|(_, def_id, _)| *def_id)
                .collect();
            FunctionWithContractPass { check_fn, replace_fns, unused_closures: Default::default() }
        } else {
            // Building the model for tests or public functions.
            FunctionWithContractPass::default()
        }
    }

    /// Functions with contract have the following structure:
    /// ```ignore
    /// fn original([self], args*) {
    ///    let kani_contract_mode = kani::internal::mode(); // ** Replace this call
    ///    match kani_contract_mode {
    ///        kani::internal::RECURSION_CHECK => {
    ///            let closure = |/*args*/|{ /*body*/};
    ///            kani_register_contract(closure) // ** Replace this call
    ///        }
    ///        kani::internal::REPLACE => {
    ///            // same as above
    ///        }
    ///        kani::internal::SIMPLE_CHECK => {
    ///            // same as above
    ///        }
    ///        _ => { /* original code */}
    ///    }
    /// }
    /// ```
    /// See function `handle_untouched` inside `kani_macros`.
    ///
    /// Thus, we need to:
    /// 1. Initialize `kani_contract_mode` variable to the value corresponding to the mode.
    ///
    /// Thus replace this call:
    /// ```ignore
    ///    let kani_contract_mode = kani::internal::mode(); // ** Replace this call
    /// ```
    /// by:
    /// ```ignore
    ///    let kani_contract_mode = mode_const;
    ///    goto bbX;
    /// ```
    /// 2. Replace `kani_register_contract` by the call to the closure.
    fn set_mode(&self, tcx: TyCtxt, body: Body, mode: ContractMode) -> Body {
        debug!(?mode, "set_mode");
        let mode_fn = find_fn_def(tcx, "KaniContractMode").unwrap();
        let mut new_body = MutableBody::from(body);
        let (mut mode_call, ret, target) = new_body
            .blocks()
            .iter()
            .enumerate()
            .find_map(|(bb_idx, bb)| {
                if let TerminatorKind::Call { func, target, destination, .. } = &bb.terminator.kind
                {
                    let (callee, _) = func.ty(new_body.locals()).unwrap().kind().fn_def()?;
                    (callee == mode_fn).then(|| {
                        (
                            SourceInstruction::Terminator { bb: bb_idx },
                            destination.clone(),
                            target.unwrap(),
                        )
                    })
                } else {
                    None
                }
            })
            .unwrap();

        let span = mode_call.span(new_body.blocks());
        let mode_const = new_body.new_uint_operand(mode as _, UintTy::U8, span);
        new_body.assign_to(
            ret.clone(),
            Rvalue::Use(mode_const),
            &mut mode_call,
            InsertPosition::Before,
        );
        new_body.replace_terminator(
            &mode_call,
            Terminator { kind: TerminatorKind::Goto { target }, span },
        );

        new_body.into()
    }

    /// Return which contract mode to use for this function if any.
    fn contract_mode(&self, tcx: TyCtxt, fn_def: FnDef) -> Option<ContractMode> {
        let kani_attributes = KaniAttributes::for_def_id(tcx, fn_def.def_id());
        kani_attributes.has_contract().then(|| {
            let fn_def_id = rustc_internal::internal(tcx, fn_def.def_id());
            if self.check_fn == Some(fn_def_id) {
                if kani_attributes.has_recursion() {
                    ContractMode::RecursiveCheck
                } else {
                    ContractMode::SimpleCheck
                }
            } else if self.replace_fns.contains(&fn_def_id) {
                ContractMode::Replace
            } else {
                ContractMode::Original
            }
        })
    }

    /// Select any unused closure for body deletion.
    fn mark_unused(&mut self, tcx: TyCtxt, fn_def: FnDef, body: &Body, mode: ContractMode) {
        let contract =
            KaniAttributes::for_def_id(tcx, fn_def.def_id()).contract_attributes().unwrap();
        let recursion_closure = find_closure(tcx, fn_def, &body, contract.recursion_check.as_str());
        let check_closure = find_closure(tcx, fn_def, &body, contract.checked_with.as_str());
        let replace_closure = find_closure(tcx, fn_def, &body, contract.replaced_with.as_str());
        match mode {
            ContractMode::Original => {
                // No contract instrumentation needed. Add all closures to the list of unused.
                self.unused_closures.insert(recursion_closure);
                self.unused_closures.insert(check_closure);
                self.unused_closures.insert(replace_closure);
            }
            ContractMode::RecursiveCheck => {
                self.unused_closures.insert(replace_closure);
                self.unused_closures.insert(check_closure);
            }
            ContractMode::SimpleCheck => {
                self.unused_closures.insert(replace_closure);
                self.unused_closures.insert(recursion_closure);
            }
            ContractMode::Replace => {
                self.unused_closures.insert(recursion_closure);
                self.unused_closures.insert(check_closure);
            }
        }
    }
}

/// Enumeration that store the value of which implementation should be selected.
///
/// Keep the discriminant values in sync with [kani::internal::mode].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ContractMode {
    Original = 0,
    RecursiveCheck = 1,
    SimpleCheck = 2,
    Replace = 3,
}

fn find_closure(tcx: TyCtxt, fn_def: FnDef, body: &Body, name: &str) -> ClosureDef {
    body.var_debug_info
        .iter()
        .find_map(|var_info| {
            if var_info.name.as_str() == name {
                let ty = match &var_info.value {
                    VarDebugInfoContents::Place(place) => place.ty(body.locals()).unwrap(),
                    VarDebugInfoContents::Const(const_op) => const_op.ty(),
                };
                if let TyKind::RigidTy(RigidTy::Closure(def, _args)) = ty.kind() {
                    return Some(def);
                }
            }
            None
        })
        .unwrap_or_else(|| {
            tcx.sess.dcx().err(format!(
                "Failed to find contract closure `{name}` in function `{}`",
                fn_def.name()
            ));
            tcx.sess.dcx().abort_if_errors();
            unreachable!()
        })
}
