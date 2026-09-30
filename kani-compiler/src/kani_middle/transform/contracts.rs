// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code related to the MIR-to-MIR pass to enable contracts.
use crate::args::ReachabilityType;
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::kani_functions::{KaniIntrinsic, KaniModel};
use crate::kani_middle::transform::body::{
    InsertPosition, MutableBody, SourceInstruction, synthetic_source_info,
};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use cbmc::{InternString, InternedString};
use rustc_middle::ty::TyCtxt;
use rustc_public::CrateDef;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{
    BinOp, Body, CastKind, ConstOperand, Mutability, Operand, Place, Rvalue, Terminator,
    TerminatorKind, VarDebugInfoContents, WithRetag,
};
use rustc_public::rustc_internal;
use rustc_public::ty::{
    ClosureDef, FnDef, GenericArgKind, GenericArgs, MirConst, RigidTy, Ty, TyKind, TypeAndMut,
    UintTy,
};
use rustc_span::Symbol;
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
#[derive(Debug, Clone)]
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
    pub fn new(tcx: TyCtxt, queries: &QueryDb, unit: &CodegenUnit) -> AnyModifiesPass {
        let kani_fns = queries.kani_functions();
        let kani_any = kani_fns.get(&KaniModel::Any.into()).copied();
        let kani_any_modifies = kani_fns.get(&KaniIntrinsic::AnyModifies.into()).copied();
        let kani_write_any = kani_fns.get(&KaniIntrinsic::WriteAny.into()).copied();
        let kani_write_any_slim = kani_fns.get(&KaniModel::WriteAnySlim.into()).copied();
        let kani_write_any_slice = kani_fns.get(&KaniModel::WriteAnySlice.into()).copied();
        let kani_write_any_str = kani_fns.get(&KaniModel::WriteAnyStr.into()).copied();
        let target_fn = if let Some(harness) = unit.harnesses.first() {
            let attributes = KaniAttributes::for_instance(tcx, *harness);
            attributes.proof_for_contract().map(|symbol| symbol.unwrap().as_str().intern())
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
                let span = bb.terminator.source_info.span;
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
                if let TyKind::RigidTy(RigidTy::Slice(elem_ty)) = internal_type.kind() {
                    //if the input is a slice `[T]`, use write_any_slice. Note that
                    //`write_any_slice<T>(slice: *mut [T])` is generic over the *element* type
                    //`T`, whereas `instance_args` holds the pointee type `[T]`. Resolving with
                    //`instance_args` here would incorrectly produce `write_any_slice<[T]>`
                    //(i.e. a `*mut [[T]]`), so we resolve with the element type instead.
                    let elem_args = GenericArgs(vec![GenericArgKind::Type(elem_ty)]);
                    let instance =
                        Instance::resolve(self.kani_write_any_slice.unwrap(), &elem_args).unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.source_info.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                } else if let TyKind::RigidTy(RigidTy::Str) = internal_type.kind() {
                    //if the input is a str, use write_any_str
                    let instance =
                        Instance::resolve(self.kani_write_any_str.unwrap(), &instance_args)
                            .unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.source_info.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                } else {
                    //otherwise, use write_any_slim
                    let instance =
                        Instance::resolve(self.kani_write_any_slim.unwrap(), &instance_args)
                            .unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.source_info.span;
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
                        let msg = if let Some(target_fn) = self.target_fn {
                            format!(
                                "`{receiver_ty}` doesn't implement `kani::Arbitrary`.\
                                        Please, check `{}` contract.",
                                target_fn,
                            )
                        } else {
                            format!("`{receiver_ty}` doesn't implement `kani::Arbitrary`.")
                        };
                        tcx.dcx()
                            .struct_span_err(
                                rustc_internal::internal(tcx, bb.terminator.source_info.span),
                                msg,
                            )
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
/// #[kanitool::asserted_with = "__kani_assert_modify"]
/// #[kanitool::modifies_wrapper = "__kani_modifies_modify"]
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
///         kani::internal::ASSERT => {
///             #[kanitool::is_contract_generated(assert)]
///             let mut __kani_check_name_fn = || { /* assert body */ };
///             kani_register_contract(__kani_assert_name_fn)
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
#[derive(Debug, Default, Clone)]
pub struct FunctionWithContractPass {
    /// Function that is being checked, if any.
    check_fn: Option<FnDef>,
    /// Functions that should be stubbed by their contract.
    replace_fns: HashSet<FnDef>,
    /// Should we interpret contracts as assertions? (true iff the no-assert-contracts option is not passed)
    assert_contracts: bool,
    /// Functions annotated with contract attributes will contain contract closures even if they
    /// are not to be used in this harness.
    /// In order to avoid bringing unnecessary logic, we clear their body.
    unused_closures: HashSet<ClosureDef>,
    /// Cache KaniRunContract function used to implement contracts.
    run_contract_fn: Option<FnDef>,
    /// Cache of the InContractClauseModel function used to dispatch calls to
    /// the function under contract verification to its contract replacement
    /// when they occur during evaluation of another contract's clauses.
    in_clause_fn: Option<FnDef>,
    /// Cache `kani::any` used to detect `Arbitrary` cycles in verified stubs.
    kani_any: Option<FnDef>,
    /// Instances we already ran the `Arbitrary` cycle check on, so we only report
    /// once each. Keyed by monomorphized instance so that distinct
    /// instantiations of a generic target are each checked.
    arbitrary_cycle_checked: HashSet<Instance>,
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
                    if mode == ContractMode::RecursiveCheck {
                        check_mutual_recursion(tcx, *def, &body);
                    }
                    // Key the dedup on the monomorphized instance, not the
                    // `FnDef`: a generic `stub_verified` target can be
                    // instantiated at several types, and each instantiation has
                    // its own return type and so its own potential cycle.
                    if mode == ContractMode::Replace
                        && self.arbitrary_cycle_checked.insert(instance)
                    {
                        self.check_arbitrary_cycle(tcx, *def, args);
                    }
                    self.mark_unused(tcx, *def, &body, mode);
                    let new_body = self.set_mode(tcx, body, mode);
                    (true, new_body)
                } else if KaniAttributes::for_instance(tcx, instance).fn_marker()
                    == Some(Symbol::intern("kani_register_contract"))
                {
                    let run = Instance::resolve(self.run_contract_fn.unwrap(), args).unwrap();
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
    pub fn new(tcx: TyCtxt, queries: &QueryDb, unit: &CodegenUnit) -> FunctionWithContractPass {
        if let Some(harness) = unit.harnesses.first() {
            let (check_fn, replace_fns) = {
                let harness_generic_args = harness.args().0;
                // Manual harnesses have no arguments, so if there are generic arguments,
                // we know this is an automatic harness
                if matches!(queries.args().reachability_analysis, ReachabilityType::AllFns)
                    && !harness_generic_args.is_empty()
                {
                    let kind = harness.args().0[0].expect_ty().kind();
                    let (fn_to_verify_def, _) = kind.fn_def().unwrap();
                    // For automatic harnesses, the target is the function to verify,
                    // and stubs are empty.
                    (Some(fn_to_verify_def), HashSet::default())
                } else {
                    let attrs = KaniAttributes::for_instance(tcx, *harness);
                    let check_fn = attrs.interpret_for_contract_attribute();
                    let replace_fns: HashSet<_> =
                        attrs.interpret_stub_verified_attribute().into_iter().collect();
                    (check_fn, replace_fns)
                }
            };
            let run_contract_fn =
                queries.kani_functions().get(&KaniModel::RunContract.into()).copied();
            assert!(run_contract_fn.is_some(), "Failed to find Kani run contract function");
            let in_clause_fn =
                queries.kani_functions().get(&KaniModel::InContractClause.into()).copied();
            assert!(in_clause_fn.is_some(), "Failed to find Kani in-contract-clause function");
            FunctionWithContractPass {
                check_fn,
                replace_fns,
                assert_contracts: !queries.args().no_assert_contracts,
                unused_closures: Default::default(),
                run_contract_fn,
                in_clause_fn,
                kani_any: queries.kani_functions().get(&KaniModel::Any.into()).copied(),
                arbitrary_cycle_checked: Default::default(),
            }
        } else {
            // If reachability mode is PubFns or Tests, we just remove any contract logic.
            // Note that in this path there is no proof harness.
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
    ///        kani::internal::ASSERT => {
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
        let mut new_body = MutableBody::from(body);
        let (mut mode_call, ret, target) = new_body
            .blocks()
            .iter()
            .enumerate()
            .find_map(|(bb_idx, bb)| {
                if let TerminatorKind::Call { func, target, destination, .. } = &bb.terminator.kind
                {
                    let (callee, _) = func.ty(new_body.locals()).unwrap().kind().fn_def()?;
                    let marker = KaniAttributes::for_def_id(tcx, callee.def_id()).fn_marker();
                    if marker.is_some_and(|s| s.as_str() == "kani_contract_mode") {
                        return Some((
                            SourceInstruction::Terminator { bb: bb_idx },
                            destination.clone(),
                            target.unwrap(),
                        ));
                    }
                }
                None
            })
            .unwrap();

        let span = mode_call.span(new_body.blocks());
        let mode_const = new_body.new_uint_operand(mode as _, UintTy::U8, span);
        if matches!(
            mode,
            ContractMode::SimpleCheck | ContractMode::RecursiveCheck | ContractMode::Assert
        ) {
            // Calls occurring during the evaluation of *contract clauses* of
            // other functions are dispatched to the original body (mode 0,
            // exact semantics) rather than the mode selected for normal
            // calls, by computing the mode at runtime as
            // `mode * (1 - in_contract_clause())`:
            //
            // * For check modes: while the harness is checking the contract
            //   of this function, the function may also be called from
            //   contract clauses of other functions in the harness's call
            //   graph (e.g. a postcondition mentioning `NonNull::as_ptr`
            //   evaluated while `as_ptr` itself is under verification). Such
            //   calls must not be dispatched to the check closure: they
            //   would consume the single top-level contract check and run
            //   write-set instrumentation in the clause's context. (Unlike
            //   dispatching to the contract replacement, the original body
            //   does not require the return type to implement Arbitrary.)
            //
            // * For assert mode: asserting the contracts of dependencies
            //   (the default since #3802) is an aid for detecting API misuse
            //   in user code; re-asserting them for calls made by *contract
            //   clauses* checks specification-level plumbing at a
            //   multiplicative cost. Clause evaluation is meant to compute a
            //   predicate over the pre-/post-states, and the functions it
            //   calls are best executed with their exact semantics (their
            //   bodies remain fully inlined and UB-checked either way).
            let in_clause_instance =
                Instance::resolve(self.in_clause_fn.unwrap(), &GenericArgs(vec![])).unwrap();
            let in_clause_local = new_body.new_local(Ty::bool_ty(), span, Mutability::Mut);
            new_body.insert_call(
                &in_clause_instance,
                &mut mode_call,
                InsertPosition::Before,
                vec![],
                Place::from(in_clause_local),
            );
            let u8_ty = Ty::from_rigid_kind(RigidTy::Uint(UintTy::U8));
            let in_clause_u8 = new_body.insert_assignment(
                Rvalue::Cast(
                    CastKind::IntToInt,
                    Operand::Move(Place::from(in_clause_local)),
                    u8_ty,
                ),
                &mut mode_call,
                InsertPosition::Before,
            );
            let one_const = new_body.new_uint_operand(1, UintTy::U8, span);
            let not_in_clause = new_body.insert_binary_op(
                BinOp::Sub,
                one_const,
                Operand::Move(Place::from(in_clause_u8)),
                &mut mode_call,
                InsertPosition::Before,
            );
            new_body.assign_to(
                ret.clone(),
                Rvalue::BinaryOp(BinOp::Mul, mode_const, Operand::Move(Place::from(not_in_clause))),
                &mut mode_call,
                InsertPosition::Before,
            );
        } else {
            new_body.assign_to(
                ret.clone(),
                Rvalue::Use(mode_const, WithRetag::No),
                &mut mode_call,
                InsertPosition::Before,
            );
        }
        new_body.replace_terminator(
            &mode_call,
            Terminator {
                kind: TerminatorKind::Goto { target },
                source_info: synthetic_source_info(span),
            },
        );

        new_body.into()
    }

    /// Return which contract mode to use for this function if any.
    /// Note that the Check and Replace modes take precedence over the Assert mode.
    /// This precedence ensures that a given `target` of a proof_for_contract(target) or stub_verified(target)
    /// use their Check or Replace closures, respectively, rather than the Assert closure.
    fn contract_mode(&self, tcx: TyCtxt, fn_def: FnDef) -> Option<ContractMode> {
        let kani_attributes = KaniAttributes::for_def_id(tcx, fn_def.def_id());
        kani_attributes.has_contract().then(|| {
            if self.check_fn == Some(fn_def) {
                if kani_attributes.has_recursion() {
                    ContractMode::RecursiveCheck
                } else {
                    ContractMode::SimpleCheck
                }
            } else if self.replace_fns.contains(&fn_def) {
                ContractMode::Replace
            } else if self.assert_contracts {
                ContractMode::Assert
            } else {
                ContractMode::Original
            }
        })
    }

    /// Detect the `stub_verified` / `Arbitrary` cycle described in
    /// <https://github.com/model-checking/kani/pull/4571>.
    ///
    /// A contract replacement havocs its own return value with
    /// `kani::any::<Ret>()` (see `initial_replace_stmts` in `kani_macros`, where
    /// `any_modifies` is emitted and later rewritten to `kani::any` by
    /// [`AnyModifiesPass`]). So if `Ret`'s `Arbitrary` implementation reaches the
    /// stubbed function again, the replacement calls itself through
    /// `Arbitrary::any`:
    ///
    /// ```text
    /// normalize -> replace closure -> kani::any::<Wrapper> ->
    ///     <Wrapper as Arbitrary>::any -> Wrapper::new -> normalize -> ...
    /// ```
    ///
    /// This recursion is unbounded and has no fixpoint, so CBMC unwinds until it
    /// exhausts memory. Report it at compile time instead, since the alternative
    /// is a silent multi-minute hang followed by an out-of-memory message that
    /// does not name the cause.
    fn check_arbitrary_cycle(&self, tcx: TyCtxt, fn_def: FnDef, args: &GenericArgs) {
        let Some(kani_any) = self.kani_any else { return };
        let Ok(instance) = Instance::resolve(fn_def, args) else { return };
        // Bail rather than ICE if the ABI is not computable; this check is
        // diagnostic-only, so failing to run it just leaves prior behavior.
        let Ok(fn_abi) = instance.fn_abi() else { return };
        let ret_ty = fn_abi.ret.ty;

        // Resolve `kani::any::<Ret>`. This fails when `Ret` does not implement
        // `Arbitrary`, which `AnyModifiesPass::any_body` already diagnoses.
        let any_args = GenericArgs(vec![GenericArgKind::Type(ret_ty)]);
        let Ok(any_instance) = Instance::resolve(kani_any, &any_args) else { return };

        let Some(path) = find_call_path(&any_instance, &instance, &mut HashSet::new()) else {
            return;
        };

        let fn_name = tcx.def_path_str(rustc_internal::internal(tcx, fn_def.def_id()));
        let span = rustc_internal::internal(tcx, fn_def.span());
        // Render one uniform `-> callee` entry per line, ending at the stubbed
        // function itself. The leading `kani::any::<Ret>` frame is dropped: the
        // note above already names it, and starting at the `Arbitrary::any` call
        // is where the recursion actually begins. Instance names are used
        // throughout so the trace is consistently crate-qualified.
        let trace = path
            .iter()
            .skip(1)
            .chain(std::iter::once(&instance.name()))
            .map(|frame| format!("    -> {frame}"))
            .collect::<Vec<_>>()
            .join("\n");
        tcx.dcx()
            .struct_span_err(
                span,
                format!(
                    "`{fn_name}` is used as a verified stub, but generating an \
                     arbitrary value of its return type `{ret_ty}` calls \
                     `{fn_name}` again"
                ),
            )
            .with_note(format!(
                "the contract replacement havocs its return value with \
                 `kani::any::<{ret_ty}>()`, so this forms an unbounded recursion:\n\
                 {trace}"
            ))
            .with_help(format!(
                "derive `Arbitrary` for `{ret_ty}` instead of implementing it manually, \
                 or avoid calling `{fn_name}` from the `Arbitrary` implementation"
            ))
            .emit();
    }

    /// Select any unused closure for body deletion.
    fn mark_unused(&mut self, tcx: TyCtxt, fn_def: FnDef, body: &Body, mode: ContractMode) {
        let contract =
            KaniAttributes::for_def_id(tcx, fn_def.def_id()).contract_attributes().unwrap();
        let recursion_closure = find_closure(tcx, fn_def, body, contract.recursion_check.as_str());
        let check_closure = find_closure(tcx, fn_def, body, contract.checked_with.as_str());
        let replace_closure = find_closure(tcx, fn_def, body, contract.replaced_with.as_str());
        let assert_closure = find_closure(tcx, fn_def, body, contract.asserted_with.as_str());
        match mode {
            ContractMode::Original => {
                // No contract instrumentation needed. Add all closures to the list of unused.
                self.unused_closures.insert(recursion_closure);
                self.unused_closures.insert(check_closure);
                self.unused_closures.insert(replace_closure);
                self.unused_closures.insert(assert_closure);
            }
            ContractMode::RecursiveCheck => {
                self.unused_closures.insert(replace_closure);
                self.unused_closures.insert(check_closure);
                self.unused_closures.insert(assert_closure);
            }
            ContractMode::SimpleCheck => {
                self.unused_closures.insert(replace_closure);
                self.unused_closures.insert(recursion_closure);
                self.unused_closures.insert(assert_closure);
            }
            ContractMode::Replace => {
                self.unused_closures.insert(recursion_closure);
                self.unused_closures.insert(check_closure);
                self.unused_closures.insert(assert_closure);
            }
            ContractMode::Assert => {
                self.unused_closures.insert(recursion_closure);
                self.unused_closures.insert(check_closure);
                self.unused_closures.insert(replace_closure);
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
    Assert = 4,
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

/// Search the call graph rooted at `from` for a path reaching `target`.
///
/// Returns the chain of function names leading to `target` (excluding `target`
/// itself) so the diagnostic can show the user the cycle. `visited` guards
/// against non-terminating traversal of recursive call graphs.
///
/// `target` is a monomorphized instance, and callees are compared against it
/// instance-precisely rather than by `DefId`. A generic function's `Arbitrary`
/// implementation may call a *different* monomorphization of that same generic
/// function; that does not re-enter the replacement instance under check, and
/// the chain may well terminate. Comparing by `DefId` alone would reject such
/// working proofs.
///
/// This is a syntactic walk over monomorphized MIR, so it only follows statically
/// resolvable calls. Calls through function pointers or trait objects are not
/// followed, meaning a cycle routed through them is not detected. That is
/// acceptable here: missing a detection reproduces today's behavior (the hang),
/// while a false positive would reject a working proof.
fn find_call_path(
    from: &Instance,
    target: &Instance,
    visited: &mut HashSet<Instance>,
) -> Option<Vec<String>> {
    if !visited.insert(*from) {
        return None;
    }
    let body = from.body()?;

    for bb in body.blocks.iter() {
        let TerminatorKind::Call { func, .. } = &bb.terminator.kind else { continue };
        let Ok(func_ty) = func.ty(body.locals()) else { continue };
        let TyKind::RigidTy(RigidTy::FnDef(callee_def, callee_args)) = func_ty.kind() else {
            continue;
        };
        let Ok(callee) = Instance::resolve(callee_def, &callee_args) else { continue };

        if callee == *target {
            return Some(vec![from.name()]);
        }

        if let Some(mut path) = find_call_path(&callee, target, visited) {
            path.insert(0, from.name());
            return Some(path);
        }
    }
    None
}

/// Check if a function with `#[kani::recursion]` is involved in mutual recursion.
///
/// Scans the function's MIR body for calls to other functions that also have
/// `#[kani::recursion]`. For each such callee, checks if the callee's body calls
/// back to the original function (one level of indirection). If so, emits a
/// compilation error because the per-function REENTRY mechanism only handles
/// direct recursion soundly.
///
/// We require both `has_contract()` and `has_recursion()` on the callee because
/// if the callee has a contract but no `#[kani::recursion]`, Kani replaces the
/// callee with its contract abstraction — no mutual recursion occurs.
///
/// Limitations:
/// - Only detects one level of indirection (f→g→f), not deeper chains (f→g→h→f).
///   TODO(#3316): Extend to detect deeper mutual recursion chains.
fn check_mutual_recursion(tcx: TyCtxt, fn_def: FnDef, body: &Body) {
    let fn_name = tcx.def_path_str(rustc_internal::internal(tcx, fn_def.def_id()));

    for bb in body.blocks.iter() {
        let TerminatorKind::Call { func, .. } = &bb.terminator.kind else { continue };
        let Ok(func_ty) = func.ty(body.locals()) else { continue };
        let TyKind::RigidTy(RigidTy::FnDef(callee_def, callee_args)) = func_ty.kind() else {
            continue;
        };

        // Skip direct recursion (that's handled correctly by REENTRY).
        if callee_def.def_id() == fn_def.def_id() {
            continue;
        }

        // Only error when the callee also uses #[kani::recursion] with a contract.
        // If the callee has a contract but no #[kani::recursion], Kani replaces
        // the call with the contract abstraction, so no mutual recursion occurs.
        let callee_attrs = KaniAttributes::for_def_id(tcx, callee_def.def_id());
        if !callee_attrs.has_contract() || !callee_attrs.has_recursion() {
            continue;
        }

        // Check if the callee calls back to us (one level deep).
        let Ok(callee_instance) = Instance::resolve(callee_def, &callee_args) else { continue };
        let Some(callee_body) = callee_instance.body() else { continue };

        for callee_bb in callee_body.blocks.iter() {
            let TerminatorKind::Call { func: callee_func, .. } = &callee_bb.terminator.kind else {
                continue;
            };
            let Ok(callee_func_ty) = callee_func.ty(callee_body.locals()) else { continue };
            let TyKind::RigidTy(RigidTy::FnDef(transitive_def, _)) = callee_func_ty.kind() else {
                continue;
            };

            if transitive_def.def_id() == fn_def.def_id() {
                let callee_name =
                    tcx.def_path_str(rustc_internal::internal(tcx, callee_def.def_id()));
                let span = rustc_internal::internal(tcx, bb.terminator.source_info.span);
                tcx.dcx().span_err(
                    span,
                    format!(
                        "`#[kani::recursion]` is used on `{fn_name}`, which calls \
                         `{callee_name}` that calls back to `{fn_name}`. \
                         Mutual recursion is not supported by contract verification \
                         and produces unsound results. Only direct recursion \
                         (a function calling itself) is handled correctly."
                    ),
                );
                break; // One error per callee is enough; continue checking other callees.
            }
        }
    }
}
