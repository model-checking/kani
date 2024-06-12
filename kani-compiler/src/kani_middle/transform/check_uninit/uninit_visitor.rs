use crate::kani_middle::transform::body::{MutableBody, SourceInstruction};
use stable_mir::mir::mono::{Instance, InstanceKind};
use stable_mir::mir::visit::{Location, PlaceContext};
use stable_mir::mir::{
    BasicBlockIdx, Constant, LocalDecl, MirVisitor, Mutability, NonDivergingIntrinsic, Operand,
    Place, ProjectionElem, Statement, StatementKind, Terminator, TerminatorKind,
};
use stable_mir::ty::{Const, RigidTy, Span, TyKind, UintTy};
use strum_macros::AsRefStr;

#[derive(AsRefStr, Clone, Debug)]
pub enum SourceOp {
    Get { place: Place, count: Operand },
    Set { place: Place, count: Operand, value: bool },
    Unsupported { instruction: String, place: Place },
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
    Operand::Constant(Constant {
        span,
        user_ty: None,
        literal: Const::try_from_uint(value as u128, UintTy::Usize).unwrap(),
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

/// Retrieve instance for the given function operand.
///
/// This will panic if the operand is not a function or if it cannot be resolved.
fn expect_instance(locals: &[LocalDecl], func: &Operand) -> Instance {
    let ty = func.ty(locals).unwrap();
    match ty.kind() {
        TyKind::RigidTy(RigidTy::FnDef(def, args)) => Instance::resolve(def, &args).unwrap(),
        _ => unreachable!(),
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
                    // Then check the destination place.
                    if let Some(place_without_deref) = try_remove_topmost_deref(place) {
                        if place_without_deref.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                            self.push_target(SourceOp::Set {
                                place: place_without_deref,
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
                TerminatorKind::Call { func, args, .. } => {
                    self.super_terminator(term, location);
                    let instance = expect_instance(self.locals, func);
                    if instance.kind == InstanceKind::Intrinsic {
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
                            _ => {}
                        }
                    }
                }
                TerminatorKind::Drop { place, .. } => {
                    self.super_terminator(term, location);
                    if place.ty(&self.locals).unwrap().kind().is_raw_ptr() {
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
                            instruction: "union access".to_string(),
                            place: intermediate_place.clone(),
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
}
