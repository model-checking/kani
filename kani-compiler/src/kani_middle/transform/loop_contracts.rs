// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains code related to the MIR-to-MIR pass to enable loop contracts.
//!

use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::find_fn_def;
use crate::kani_middle::transform::body::{MutableBody, SourceInstruction};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_middle::KaniAttributes;
use crate::kani_queries::QueryDb;
use crate::stable_mir::CrateDef;
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{BasicBlockIdx, Body, Operand, Terminator, TerminatorKind};
use stable_mir::ty::{FnDef, RigidTy};
use stable_mir::DefId;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use tracing::trace;

/// This pass will perform the following operations:
/// 1. Replace the body of `kani_register_loop_contract` by `kani::internal::run_contract_fn`
///    to invoke the closure.
///
/// 2. Replace the dummy call to the register function with the actual call, i.e., transform
///
/// ```ignore
/// let kani_loop_invariant = || -> bool {inv};
/// kani_register_loop_contract(kani_loop_invariant)
/// while guard {
///     loop_body;
///     kani_register_loop_contract(||->bool{true});
/// }
/// ```
///
///    to
///
/// ```ignore
/// let kani_loop_invariant = || -> bool {inv};
/// while guard {
///     loop_body;
///     kani_register_loop_contract(kani_loop_invariant);
/// }
///
/// ```
///
/// 3. Move the call to the register function to the loop latch terminator. This is required
///    as in MIR, there could be some `StorageDead` statements between register calls and
///    loop latches.
#[derive(Debug, Default)]
pub struct FunctionWithLoopContractPass {
    /// Cache KaniRunContract function used to implement contracts.
    run_contract_fn: Option<FnDef>,
    /// Function and Arguments of register functions.
    registered_args: HashMap<DefId, (Operand, Vec<Operand>)>,
    /// The terminator we are moving to the loop latch.
    loop_terminator: Option<Terminator>,
}

impl TransformPass for FunctionWithLoopContractPass {
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
        trace!(function=?instance.name(), "FunctionWithLoopContractPass::transform");
        match instance.ty().kind().rigid().unwrap() {
            RigidTy::FnDef(_func, args) => {
                if KaniAttributes::for_instance(tcx, instance).fn_marker()
                    == Some(Symbol::intern("kani_register_loop_contract"))
                {
                    // Replace the body of the register function with `run_contract_fn`'s.
                    let run = Instance::resolve(self.run_contract_fn.unwrap(), args).unwrap();
                    (true, run.body().unwrap())
                } else {
                    // Replace the dummy register call with the actual register call.
                    let mut new_body = MutableBody::from(body);
                    let mut contain_loop_contracts: bool = false;

                    // Visit basic blocks in control flow order.
                    let mut visited: HashSet<BasicBlockIdx> = HashSet::new();
                    let mut queue: VecDeque<BasicBlockIdx> = VecDeque::new();
                    queue.push_back(0);

                    while let Some(bbidx) = queue.pop_front() {
                        visited.insert(bbidx);
                        // We only need to transform basic block with terminators as calls
                        // to the register functions, no matter dummy or actual calls.
                        let terminator = new_body.blocks()[bbidx].terminator.clone();
                        if let TerminatorKind::Call {
                            func: terminator_func,
                            args: terminator_args,
                            destination,
                            target,
                            unwind,
                        } = &terminator.kind
                        {
                            // Get the function signature of the terminator call.
                            let fn_kind = match terminator_func.ty(new_body.locals()) {
                                Ok(fn_ty) => fn_ty.kind(),
                                _ => continue,
                            };
                            let RigidTy::FnDef(fn_def, ..) = fn_kind.rigid().unwrap() else {
                                continue;
                            };

                            if KaniAttributes::for_def_id(tcx, fn_def.def_id()).fn_marker()
                                == Some(Symbol::intern("kani_register_loop_contract"))
                            {
                                contain_loop_contracts = true;

                                if let std::collections::hash_map::Entry::Occupied(entry) =
                                    self.registered_args.entry(fn_def.def_id())
                                {
                                    // This call is a dummy call as it is not the first call
                                    // to the register function.
                                    // Replace it with `self.loop_terminator`.
                                    self.loop_terminator = Some(Terminator {
                                        kind: TerminatorKind::Call {
                                            func: entry.get().0.clone(),
                                            args: entry.get().1.to_vec(),
                                            destination: destination.clone(),
                                            target: *target,
                                            unwind: *unwind,
                                        },
                                        span: terminator.span,
                                    });
                                    new_body.replace_terminator(
                                        &SourceInstruction::Terminator { bb: bbidx },
                                        Terminator {
                                            kind: TerminatorKind::Goto { target: target.unwrap() },
                                            span: terminator.span,
                                        },
                                    );
                                    // Then move the loop terminator to the loop latch.
                                    self.move_loop_terminator_to_loop_latch(
                                        bbidx,
                                        &mut new_body,
                                        &mut visited,
                                    );
                                } else {
                                    // This call is an actual call as it is the first call
                                    // to the register function.
                                    self.registered_args.insert(
                                        fn_def.def_id(),
                                        (terminator_func.clone(), terminator_args.clone()),
                                    );
                                    new_body.replace_terminator(
                                        &SourceInstruction::Terminator { bb: bbidx },
                                        Terminator {
                                            kind: TerminatorKind::Goto { target: target.unwrap() },
                                            span: terminator.span,
                                        },
                                    );
                                }
                            }
                        }

                        // Add successors of the current basic blocks to
                        // the visiting queue.
                        for to_visit in terminator.successors() {
                            if visited.contains(&to_visit) {
                                continue;
                            }
                            queue.push_back(to_visit);
                        }
                    }
                    (contain_loop_contracts, new_body.into())
                }
            }
            _ => {
                /* static variables case */
                (false, body)
            }
        }
    }
}

impl FunctionWithLoopContractPass {
    pub fn new(tcx: TyCtxt, unit: &CodegenUnit) -> FunctionWithLoopContractPass {
        if let Some(_harness) = unit.harnesses.first() {
            let run_contract_fn = find_fn_def(tcx, "KaniRunContract");
            assert!(run_contract_fn.is_some(), "Failed to find Kani run contract function");
            FunctionWithLoopContractPass {
                run_contract_fn,
                registered_args: HashMap::new(),
                loop_terminator: None,
            }
        } else {
            // If reachability mode is PubFns or Tests, we just remove any loop contract logic.
            // Note that in this path there is no proof harness.
            FunctionWithLoopContractPass::default()
        }
    }

    //  Replace the next loop latch---a terminator that targets some basic block in `visited`---
    //  with `self.loop_terminator`.
    //  We assume that there is no branching terminator with more than one targets between the
    //  current basic block `bbidx` and the next loop latch.
    fn move_loop_terminator_to_loop_latch(
        &mut self,
        bbidx: BasicBlockIdx,
        new_body: &mut MutableBody,
        visited: &mut HashSet<BasicBlockIdx>,
    ) {
        let mut current_bbidx = bbidx;
        while self.loop_terminator.is_some() {
            if new_body.blocks()[current_bbidx].terminator.successors().len() != 1 {
                // Assume that there is no branching between the register function cal
                // and the loop latch.
                unreachable!()
            }
            let target = new_body.blocks()[current_bbidx].terminator.successors()[0];

            if visited.contains(&target) {
                // Current basic block is the loop latch.
                let Some(Terminator {
                    kind:
                        TerminatorKind::Call {
                            func: ref loop_terminator_func,
                            args: ref loop_terminator_args,
                            destination: ref loop_terminator_destination,
                            target: _loop_terminator_target,
                            unwind: ref loop_terminator_unwind,
                        },
                    span: loop_terminator_span,
                }) = self.loop_terminator
                else {
                    unreachable!()
                };
                new_body.replace_terminator(
                    &SourceInstruction::Terminator { bb: current_bbidx },
                    Terminator {
                        kind: TerminatorKind::Call {
                            func: loop_terminator_func.clone(),
                            args: loop_terminator_args.clone(),
                            destination: loop_terminator_destination.clone(),
                            target: Some(target),
                            unwind: *loop_terminator_unwind,
                        },
                        span: loop_terminator_span,
                    },
                );
                self.loop_terminator = None;
            } else {
                visited.insert(target);
                current_bbidx = target;
            }
        }
    }
}
