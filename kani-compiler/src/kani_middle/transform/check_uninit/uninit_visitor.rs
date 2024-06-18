// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Visitor that collects all instructions relevant to uninitialized memory access.

use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use stable_mir::mir::alloc::GlobalAlloc;
use stable_mir::mir::mono::{Instance, InstanceKind};
use stable_mir::mir::visit::{Location, PlaceContext};
use stable_mir::mir::{
    BasicBlockIdx, CastKind, ConstOperand, LocalDecl, MirVisitor, Mutability,
    NonDivergingIntrinsic, Operand, Place, PointerCoercion, ProjectionElem, Rvalue, Statement,
    StatementKind, Terminator, TerminatorKind,
};
use stable_mir::ty::{ConstantKind, MirConst, RigidTy, Span, TyKind, UintTy};
use strum_macros::AsRefStr;

#[derive(AsRefStr, Clone, Debug)]
pub enum SourceOp {
    Get { place: Place, count: Operand },
    Set { place: Place, count: Operand, value: bool },
    BlessConst { constant: ConstOperand, count: Operand, value: bool },
    BlessRef { place: Place, count: Operand, value: bool },
    Unsupported { reason: String },
}

impl SourceOp {
    pub fn mk_operand(&self, body: &mut MutableBody, source: &mut SourceInstruction) -> Operand {
        match self {
            SourceOp::Get { place, .. } | SourceOp::Set { place, .. } => {
                Operand::Copy(place.clone())
            }
            SourceOp::BlessRef { place, .. } => Operand::Copy(Place {
                local: body.new_assignment(
                    Rvalue::AddressOf(Mutability::Not, place.clone()),
                    source,
                    self.position(),
                ),
                projection: vec![],
            }),
            SourceOp::BlessConst { constant, .. } => Operand::Constant(constant.clone()),
            SourceOp::Unsupported { .. } => unreachable!(),
        }
    }

    pub fn expect_count(&self) -> Operand {
        match self {
            SourceOp::Get { count, .. }
            | SourceOp::Set { count, .. }
            | SourceOp::BlessConst { count, .. }
            | SourceOp::BlessRef { count, .. } => count.clone(),
            SourceOp::Unsupported { .. } => unreachable!(),
        }
    }

    pub fn position(&self) -> InsertPosition {
        match self {
            SourceOp::Get { .. }
            | SourceOp::BlessConst { .. }
            | SourceOp::BlessRef { .. }
            | SourceOp::Unsupported { .. } => InsertPosition::Before,
            SourceOp::Set { .. } => InsertPosition::After,
        }
    }

    pub fn should_be_inserted_before(&self) -> bool {
        self.position() == InsertPosition::Before
    }
}

#[derive(Clone, Debug)]
pub struct InitRelevantInstruction {
    /// The instruction that affects the state of the memory.
    pub source: SourceInstruction,
    /// All memory-related operations in this instructions.
    pub operations: Vec<SourceOp>,
}

pub struct CheckUninitVisitor<'a> {
    locals: &'a [LocalDecl],
    /// Whether we should skip the next instruction, since it might've been instrumented already.
    /// When we instrument an instruction, we partition the basic block, and the instruction that
    /// may trigger UB becomes the first instruction of the basic block, which we need to skip
    /// later.
    skip_next: bool,
    /// The instruction being visited at a given point.
    current: SourceInstruction,
    /// The target instruction that should be verified.
    pub target: Option<InitRelevantInstruction>,
    /// The basic block being visited.
    bb: BasicBlockIdx,
}

fn expect_place(op: &Operand) -> &Place {
    match op {
        Operand::Copy(place) | Operand::Move(place) => place,
        Operand::Constant(_) => unreachable!(),
    }
}

fn mk_const_operand(value: usize, span: Span) -> Operand {
    Operand::Constant(ConstOperand {
        span,
        user_ty: None,
        const_: MirConst::try_from_uint(value as u128, UintTy::Usize).unwrap(),
    })
}

fn try_remove_topmost_deref(place: &Place) -> Option<Place> {
    let mut new_place = place.clone();
    if let Some(ProjectionElem::Deref) = new_place.projection.pop() {
        Some(new_place)
    } else {
        None
    }
}

/// Try retrieving instance for the given function operand.
fn try_resolve_instance(locals: &[LocalDecl], func: &Operand) -> Result<Instance, String> {
    let ty = func.ty(locals).unwrap();
    match ty.kind() {
        TyKind::RigidTy(RigidTy::FnDef(def, args)) => Ok(Instance::resolve(def, &args).unwrap()),
        _ => Err(format!(
            "Kani does not support reasoning about memory initialization of arguments to `{ty:?}`."
        )),
    }
}

impl<'a> CheckUninitVisitor<'a> {
    pub fn find_next(
        body: &'a MutableBody,
        bb: BasicBlockIdx,
        skip_first: bool,
    ) -> Option<InitRelevantInstruction> {
        let mut visitor = CheckUninitVisitor {
            locals: body.locals(),
            skip_next: skip_first,
            current: SourceInstruction::Statement { idx: 0, bb },
            target: None,
            bb,
        };
        visitor.visit_basic_block(&body.blocks()[bb]);
        visitor.target
    }

    fn push_target(&mut self, op: SourceOp) {
        let target = self.target.get_or_insert_with(|| InitRelevantInstruction {
            source: self.current,
            operations: vec![],
        });
        target.operations.push(op);
    }
}

impl<'a> MirVisitor for CheckUninitVisitor<'a> {
    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        if self.skip_next {
            self.skip_next = false;
        } else if self.target.is_none() {
            // Leave it as an exhaustive match to be notified when a new kind is added.
            match &stmt.kind {
                StatementKind::Intrinsic(NonDivergingIntrinsic::CopyNonOverlapping(copy)) => {
                    self.super_statement(stmt, location);
                    // Source is a *const T and it must be initialized.
                    self.push_target(SourceOp::Get {
                        place: expect_place(&copy.src).clone(),
                        count: copy.count.clone(),
                    });
                    // Destimation is a *mut T so it gets initialized.
                    self.push_target(SourceOp::Set {
                        place: expect_place(&copy.dst).clone(),
                        count: copy.count.clone(),
                        value: true,
                    });
                }
                StatementKind::Assign(place, rvalue) => {
                    // First check rvalue.
                    self.visit_rvalue(rvalue, location);
                    // Check whether we are assigning into a dereference (*ptr = _).
                    if let Some(place_without_deref) = try_remove_topmost_deref(place) {
                        if place_without_deref.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                            self.push_target(SourceOp::Set {
                                place: place_without_deref,
                                count: mk_const_operand(1, location.span()),
                                value: true,
                            });
                        }
                    }
                    // Check whether Rvalue creates a new initialized pointer previously not captured inside shadow memory.
                    if place.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                        if let Rvalue::AddressOf(..) = rvalue {
                            self.push_target(SourceOp::Set {
                                place: place.clone(),
                                count: mk_const_operand(1, location.span()),
                                value: true,
                            });
                        }
                    }
                }
                StatementKind::Deinit(place) => {
                    self.super_statement(stmt, location);
                    self.push_target(SourceOp::Set {
                        place: place.clone(),
                        count: mk_const_operand(1, location.span()),
                        value: false,
                    });
                }
                StatementKind::FakeRead(_, _)
                | StatementKind::SetDiscriminant { .. }
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Retag(_, _)
                | StatementKind::PlaceMention(_)
                | StatementKind::AscribeUserType { .. }
                | StatementKind::Coverage(_)
                | StatementKind::ConstEvalCounter
                | StatementKind::Intrinsic(NonDivergingIntrinsic::Assume(_))
                | StatementKind::Nop => self.super_statement(stmt, location),
            }
        }
        let SourceInstruction::Statement { idx, bb } = self.current else { unreachable!() };
        self.current = SourceInstruction::Statement { idx: idx + 1, bb };
    }
    fn visit_terminator(&mut self, term: &Terminator, location: Location) {
        if !(self.skip_next || self.target.is_some()) {
            self.current = SourceInstruction::Terminator { bb: self.bb };
            // Leave it as an exhaustive match to be notified when a new kind is added.
            match &term.kind {
                TerminatorKind::Call { func, args, destination, .. } => {
                    self.super_terminator(term, location);
                    let instance = match try_resolve_instance(self.locals, func) {
                        Ok(instance) => instance,
                        Err(reason) => {
                            self.super_terminator(term, location);
                            self.push_target(SourceOp::Unsupported { reason });
                            return;
                        }
                    };
                    match instance.kind {
                        InstanceKind::Intrinsic => {
                            match instance.intrinsic_name().unwrap().as_str() {
                                "write_bytes" => {
                                    assert_eq!(
                                        args.len(),
                                        3,
                                        "Unexpected number of arguments for `write_bytes`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    self.push_target(SourceOp::Set {
                                        place: expect_place(&args[0]).clone(),
                                        count: args[2].clone(),
                                        value: true,
                                    })
                                }
                                "compare_bytes" => {
                                    assert_eq!(
                                        args.len(),
                                        3,
                                        "Unexpected number of arguments for `compare_bytes`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    assert!(matches!(
                                        args[1].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    self.push_target(SourceOp::Get {
                                        place: expect_place(&args[0]).clone(),
                                        count: args[2].clone(),
                                    });
                                    self.push_target(SourceOp::Get {
                                        place: expect_place(&args[1]).clone(),
                                        count: args[2].clone(),
                                    });
                                }
                                "transmute" | "transmute_copy" => {
                                    unreachable!("Should've been lowered")
                                }
                                _ => { /* TODO: add other intrinsics */ }
                            }
                        }
                        InstanceKind::Item => {
                            if instance.is_foreign_item() {
                                match instance.name().as_str() {
                                    /* TODO: implement those */
                                    "alloc::alloc::__rust_alloc"
                                    | "alloc::alloc::__rust_realloc" => {
                                        /* Memory is uninitialized, nothing to do here. */
                                    }
                                    "alloc::alloc::__rust_alloc_zeroed" => {
                                        /* Memory is initialized here, need to update shadow memory. */
                                        self.push_target(SourceOp::Set {
                                            place: destination.clone(),
                                            count: args[0].clone(),
                                            value: true,
                                        });
                                    }
                                    "alloc::alloc::__rust_dealloc" => {
                                        /* Memory is uninitialized here, need to update shadow memory. */
                                        self.push_target(SourceOp::Set {
                                            place: expect_place(&args[0]).clone(),
                                            count: args[1].clone(),
                                            value: false,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                TerminatorKind::Drop { place, .. } => {
                    self.super_terminator(term, location);
                    let place_ty = place.ty(&self.locals).unwrap();
                    // When drop is codegen'ed, a reference is taken to the place which is later implicitly coerced to a pointer.
                    // Hence, we need to bless this pointer as initialized.
                    self.push_target(SourceOp::BlessRef {
                        place: place.clone(),
                        count: mk_const_operand(1, location.span()),
                        value: true,
                    });
                    if place_ty.kind().is_raw_ptr() {
                        self.push_target(SourceOp::Set {
                            place: place.clone(),
                            count: mk_const_operand(1, location.span()),
                            value: false,
                        });
                    }
                }
                TerminatorKind::Goto { .. }
                | TerminatorKind::SwitchInt { .. }
                | TerminatorKind::Resume
                | TerminatorKind::Abort
                | TerminatorKind::Return
                | TerminatorKind::Unreachable
                | TerminatorKind::Assert { .. }
                | TerminatorKind::InlineAsm { .. } => self.super_terminator(term, location),
            }
        }
    }

    fn visit_place(&mut self, place: &Place, ptx: PlaceContext, location: Location) {
        for (idx, elem) in place.projection.iter().enumerate() {
            let intermediate_place =
                Place { local: place.local, projection: place.projection[..idx].to_vec() };
            match elem {
                ProjectionElem::Deref => {
                    let ptr_ty = intermediate_place.ty(self.locals).unwrap();
                    if ptr_ty.kind().is_raw_ptr() {
                        self.push_target(SourceOp::Get {
                            place: intermediate_place.clone(),
                            count: mk_const_operand(1, location.span()),
                        });
                    }
                }
                ProjectionElem::Field(idx, target_ty) => {
                    if target_ty.kind().is_union()
                        && (!ptx.is_mutating() || place.projection.len() > idx + 1)
                    {
                        self.push_target(SourceOp::Unsupported {
                            reason: "Kani does not support reasoning about memory initialization of unions.".to_string(),
                        });
                    }
                }
                ProjectionElem::Index(_)
                | ProjectionElem::ConstantIndex { .. }
                | ProjectionElem::Subslice { .. } => {
                    /* For a slice to be indexed, it should be valid first. */
                }
                ProjectionElem::Downcast(_) => {}
                ProjectionElem::OpaqueCast(_) => {}
                ProjectionElem::Subtype(_) => {}
            }
        }
        self.super_place(place, ptx, location)
    }

    fn visit_operand(&mut self, operand: &Operand, location: Location) {
        if let Operand::Constant(constant) = operand {
            if let ConstantKind::Allocated(allocation) = constant.const_.kind() {
                for (_, prov) in &allocation.provenance.ptrs {
                    if let GlobalAlloc::Static(_) = GlobalAlloc::from(prov.0) {
                        self.push_target(SourceOp::BlessConst {
                            constant: constant.clone(),
                            count: mk_const_operand(1, location.span()),
                            value: true,
                        });
                    };
                }
            }
        }
        self.super_operand(operand, location);
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: Location) {
        if let Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::Unsize), _, _) = rvalue {
            self.push_target(SourceOp::Unsupported {
                reason: "Kani does not support reasoning about memory initialization of unsized pointers.".to_string(),
            });
        };
        self.super_rvalue(rvalue, location);
    }
}
