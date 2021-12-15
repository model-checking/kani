//! Lowers intrinsic calls

use crate::MirPass;
use rustc_middle::mir::*;
use rustc_middle::ty::subst::SubstsRef;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::symbol::{sym, Symbol};
use rustc_span::Span;
use rustc_target::spec::abi::Abi;

pub struct LowerIntrinsics;

impl<'tcx> MirPass<'tcx> for LowerIntrinsics {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        let (basic_blocks, local_decls) = body.basic_blocks_and_local_decls_mut();
        for block in basic_blocks {
            let terminator = block.terminator.as_mut().unwrap();
            if let TerminatorKind::Call { func, args, destination, .. } = &mut terminator.kind {
                let func_ty = func.ty(local_decls, tcx);
                let (intrinsic_name, substs) = match resolve_rust_intrinsic(tcx, func_ty) {
                    None => continue,
                    Some(it) => it,
                };
                match intrinsic_name {
                    sym::unreachable => {
                        terminator.kind = TerminatorKind::Unreachable;
                    }
                    sym::forget => {
                        if let Some((destination, target)) = *destination {
                            block.statements.push(Statement {
                                source_info: terminator.source_info,
                                kind: StatementKind::Assign(Box::new((
                                    destination,
                                    Rvalue::Use(Operand::Constant(Box::new(Constant {
                                        span: terminator.source_info.span,
                                        user_ty: None,
                                        literal: ty::Const::zero_sized(tcx, tcx.types.unit).into(),
                                    }))),
                                ))),
                            });
                            terminator.kind = TerminatorKind::Goto { target };
                        }
                    }
                    sym::copy_nonoverlapping => {
                        let target = destination.unwrap().1;
                        let mut args = args.drain(..);
                        block.statements.push(Statement {
                            source_info: terminator.source_info,
                            kind: StatementKind::CopyNonOverlapping(Box::new(
                                rustc_middle::mir::CopyNonOverlapping {
                                    src: args.next().unwrap(),
                                    dst: args.next().unwrap(),
                                    count: args.next().unwrap(),
                                },
                            )),
                        });
                        assert_eq!(
                            args.next(),
                            None,
                            "Extra argument for copy_non_overlapping intrinsic"
                        );
                        drop(args);
                        terminator.kind = TerminatorKind::Goto { target };
                    }
                    sym::wrapping_add | sym::wrapping_sub | sym::wrapping_mul => {
                        if let Some((destination, target)) = *destination {
                            let lhs;
                            let rhs;
                            {
                                let mut args = args.drain(..);
                                lhs = args.next().unwrap();
                                rhs = args.next().unwrap();
                            }
                            let bin_op = match intrinsic_name {
                                sym::wrapping_add => BinOp::Add,
                                sym::wrapping_sub => BinOp::Sub,
                                sym::wrapping_mul => BinOp::Mul,
                                _ => bug!("unexpected intrinsic"),
                            };
                            block.statements.push(Statement {
                                source_info: terminator.source_info,
                                kind: StatementKind::Assign(Box::new((
                                    destination,
                                    Rvalue::BinaryOp(bin_op, Box::new((lhs, rhs))),
                                ))),
                            });
                            terminator.kind = TerminatorKind::Goto { target };
                        }
                    }
                    sym::add_with_overflow | sym::sub_with_overflow | sym::mul_with_overflow => {
                        // The checked binary operations are not suitable target for lowering here,
                        // since their semantics depend on the value of overflow-checks flag used
                        // during codegen. Issue #35310.
                    }
                    sym::size_of | sym::min_align_of => {
                        if let Some((destination, target)) = *destination {
                            let tp_ty = substs.type_at(0);
                            let null_op = match intrinsic_name {
                                sym::size_of => NullOp::SizeOf,
                                sym::min_align_of => NullOp::AlignOf,
                                _ => bug!("unexpected intrinsic"),
                            };
                            block.statements.push(Statement {
                                source_info: terminator.source_info,
                                kind: StatementKind::Assign(Box::new((
                                    destination,
                                    Rvalue::NullaryOp(null_op, tp_ty),
                                ))),
                            });
                            terminator.kind = TerminatorKind::Goto { target };
                        }
                    }
                    sym::discriminant_value => {
                        if let (Some((destination, target)), Some(arg)) =
                            (*destination, args[0].place())
                        {
                            let arg = tcx.mk_place_deref(arg);
                            block.statements.push(Statement {
                                source_info: terminator.source_info,
                                kind: StatementKind::Assign(Box::new((
                                    destination,
                                    Rvalue::Discriminant(arg),
                                ))),
                            });
                            terminator.kind = TerminatorKind::Goto { target };
                        }
                    }
                    _ if intrinsic_name.as_str().starts_with("simd_shuffle") => {
                        validate_simd_shuffle(tcx, args, terminator.source_info.span);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn resolve_rust_intrinsic<'tcx>(
    tcx: TyCtxt<'tcx>,
    func_ty: Ty<'tcx>,
) -> Option<(Symbol, SubstsRef<'tcx>)> {
    if let ty::FnDef(def_id, substs) = *func_ty.kind() {
        let fn_sig = func_ty.fn_sig(tcx);
        if let Abi::RustIntrinsic | Abi::PlatformIntrinsic = fn_sig.abi() {
            return Some((tcx.item_name(def_id), substs));
        }
    }
    None
}

fn validate_simd_shuffle<'tcx>(tcx: TyCtxt<'tcx>, args: &[Operand<'tcx>], span: Span) {
    match &args[2] {
        Operand::Constant(_) => {} // all good
        _ => {
            let msg = "last argument of `simd_shuffle` is required to be a `const` item";
            tcx.sess.span_err(span, msg);
        }
    }
}
