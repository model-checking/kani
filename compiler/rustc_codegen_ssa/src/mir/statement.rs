use rustc_errors::struct_span_err;
use rustc_middle::mir;

use super::FunctionCx;
use super::LocalRef;
use super::OperandValue;
use crate::traits::BuilderMethods;
use crate::traits::*;

impl<'a, 'tcx, Bx: BuilderMethods<'a, 'tcx>> FunctionCx<'a, 'tcx, Bx> {
    pub fn codegen_statement(&mut self, mut bx: Bx, statement: &mir::Statement<'tcx>) -> Bx {
        debug!("codegen_statement(statement={:?})", statement);

        self.set_debug_loc(&mut bx, statement.source_info);
        match statement.kind {
            mir::StatementKind::Assign(box (ref place, ref rvalue)) => {
                if let Some(index) = place.as_local() {
                    match self.locals[index] {
                        LocalRef::Place(cg_dest) => self.codegen_rvalue(bx, cg_dest, rvalue),
                        LocalRef::UnsizedPlace(cg_indirect_dest) => {
                            self.codegen_rvalue_unsized(bx, cg_indirect_dest, rvalue)
                        }
                        LocalRef::Operand(None) => {
                            let (mut bx, operand) = self.codegen_rvalue_operand(bx, rvalue);
                            self.locals[index] = LocalRef::Operand(Some(operand));
                            self.debug_introduce_local(&mut bx, index);
                            bx
                        }
                        LocalRef::Operand(Some(op)) => {
                            if !op.layout.is_zst() {
                                span_bug!(
                                    statement.source_info.span,
                                    "operand {:?} already assigned",
                                    rvalue
                                );
                            }

                            // If the type is zero-sized, it's already been set here,
                            // but we still need to make sure we codegen the operand
                            self.codegen_rvalue_operand(bx, rvalue).0
                        }
                    }
                } else {
                    let cg_dest = self.codegen_place(&mut bx, place.as_ref());
                    self.codegen_rvalue(bx, cg_dest, rvalue)
                }
            }
            mir::StatementKind::SetDiscriminant { box ref place, variant_index } => {
                self.codegen_place(&mut bx, place.as_ref())
                    .codegen_set_discr(&mut bx, variant_index);
                bx
            }
            mir::StatementKind::StorageLive(local) => {
                if let LocalRef::Place(cg_place) = self.locals[local] {
                    cg_place.storage_live(&mut bx);
                } else if let LocalRef::UnsizedPlace(cg_indirect_place) = self.locals[local] {
                    cg_indirect_place.storage_live(&mut bx);
                }
                bx
            }
            mir::StatementKind::StorageDead(local) => {
                if let LocalRef::Place(cg_place) = self.locals[local] {
                    cg_place.storage_dead(&mut bx);
                } else if let LocalRef::UnsizedPlace(cg_indirect_place) = self.locals[local] {
                    cg_indirect_place.storage_dead(&mut bx);
                }
                bx
            }
            mir::StatementKind::LlvmInlineAsm(ref asm) => {
                let outputs = asm
                    .outputs
                    .iter()
                    .map(|output| self.codegen_place(&mut bx, output.as_ref()))
                    .collect();

                let input_vals = asm.inputs.iter().fold(
                    Vec::with_capacity(asm.inputs.len()),
                    |mut acc, (span, input)| {
                        let op = self.codegen_operand(&mut bx, input);
                        if let OperandValue::Immediate(_) = op.val {
                            acc.push(op.immediate());
                        } else {
                            struct_span_err!(
                                bx.sess(),
                                span.to_owned(),
                                E0669,
                                "invalid value for constraint in inline assembly"
                            )
                            .emit();
                        }
                        acc
                    },
                );

                if input_vals.len() == asm.inputs.len() {
                    let res = bx.codegen_llvm_inline_asm(
                        &asm.asm,
                        outputs,
                        input_vals,
                        statement.source_info.span,
                    );
                    if !res {
                        struct_span_err!(
                            bx.sess(),
                            statement.source_info.span,
                            E0668,
                            "malformed inline assembly"
                        )
                        .emit();
                    }
                }
                bx
            }
            mir::StatementKind::Coverage(box ref coverage) => {
                self.codegen_coverage(&mut bx, coverage.clone(), statement.source_info.scope);
                bx
            }
            mir::StatementKind::CopyNonOverlapping(box mir::CopyNonOverlapping {
                ref src,
                ref dst,
                ref count,
            }) => {
                let dst_val = self.codegen_operand(&mut bx, dst);
                let src_val = self.codegen_operand(&mut bx, src);
                let count = self.codegen_operand(&mut bx, count).immediate();
                let pointee_layout = dst_val
                    .layout
                    .pointee_info_at(&bx, rustc_target::abi::Size::ZERO)
                    .expect("Expected pointer");
                let bytes = bx.mul(count, bx.const_usize(pointee_layout.size.bytes()));

                let align = pointee_layout.align;
                let dst = dst_val.immediate();
                let src = src_val.immediate();
                bx.memcpy(dst, align, src, align, bytes, crate::MemFlags::empty());
                bx
            }
            mir::StatementKind::FakeRead(..)
            | mir::StatementKind::Retag { .. }
            | mir::StatementKind::AscribeUserType(..)
            | mir::StatementKind::Nop => bx,
        }
    }
}
