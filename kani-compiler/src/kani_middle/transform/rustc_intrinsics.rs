// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Module responsible for implementing a few Rust compiler intrinsics.
//!
//! Note that some rustc intrinsics are lowered to MIR instructions. Those can also be handled
//! here.

use crate::intrinsics::Intrinsic;
use crate::kani_middle::kani_functions::{KaniFunction, KaniModel};
use crate::kani_middle::transform::body::{
    InsertPosition, MutMirVisitor, MutableBody, SourceInstruction,
};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{
    BasicBlockIdx, BinOp, Body, ConstOperand, LocalDecl, Operand, Rvalue, StatementKind,
    Terminator, TerminatorKind,
};
use rustc_public::rustc_internal;
use rustc_public::ty::{
    FnDef, GenericArgKind, GenericArgs, IntTy, MirConst, RigidTy, Span, Ty, TyKind, UintTy,
};
use std::collections::HashMap;
use tracing::debug;

/// Generate the body for a few Kani intrinsics.
#[derive(Debug, Clone)]
pub struct RustcIntrinsicsPass {
    /// Used to cache FnDef lookups for intrinsics models.
    models: HashMap<KaniModel, FnDef>,
}

impl TransformPass for RustcIntrinsicsPass {
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

    /// Transform the function body by inserting checks one-by-one.
    /// For every unsafe dereference or a transmute operation, we check all values are valid.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        debug!(function=?instance.name(), "transform");

        let mut new_body = MutableBody::from(body);
        let mut visitor =
            ReplaceIntrinsicCallVisitor::new(&self.models, new_body.locals().to_vec(), tcx);
        visitor.visit_body(&mut new_body);
        let changed = self.replace_lowered_intrinsics(tcx, &mut new_body);
        (visitor.changed || changed, new_body.into())
    }
}

fn is_panic_function(tcx: &TyCtxt, def_id: rustc_public::DefId) -> bool {
    let def_id = rustc_internal::internal(*tcx, def_id);
    Some(def_id) == tcx.lang_items().panic_fn()
        || tcx.has_attr(def_id, rustc_span::sym::rustc_const_panic_str)
        || Some(def_id) == tcx.lang_items().panic_fmt()
        || Some(def_id) == tcx.lang_items().begin_panic_fn()
}

impl RustcIntrinsicsPass {
    pub fn new(queries: &QueryDb) -> Self {
        let models = queries
            .kani_functions()
            .iter()
            .filter_map(|(func, def)| {
                if let KaniFunction::Model(model) = func { Some((*model, *def)) } else { None }
            })
            .collect();
        debug!(?models, "RustcIntrinsicsPass::new");
        RustcIntrinsicsPass { models }
    }

    /// This function checks if we need to replace intrinsics that have been lowered.
    fn replace_lowered_intrinsics(&self, tcx: TyCtxt, body: &mut MutableBody) -> bool {
        // Do a reverse iteration on the instructions since we will replace Rvalues by a function
        // call, which will split the basic block.
        let mut changed = false;
        let orig_bbs = body.blocks().len();
        for bb in (0..orig_bbs).rev() {
            let num_stmts = body.blocks()[bb].statements.len();
            for stmt in (0..num_stmts).rev() {
                changed |= self.replace_offset(tcx, body, bb, stmt);
            }
        }
        changed
    }

    /// Replace a lowered offset intrinsic.
    fn replace_offset(
        &self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        bb: BasicBlockIdx,
        stmt: usize,
    ) -> bool {
        let statement = &body.blocks()[bb].statements[stmt];
        let StatementKind::Assign(place, rvalue) = &statement.kind else {
            return false;
        };
        let Rvalue::BinaryOp(BinOp::Offset, op1, op2) = rvalue else { return false };
        let mut source = SourceInstruction::Statement { idx: stmt, bb };

        // Double check input parameters of `offset` operation.
        let offset_ty = op2.ty(body.locals()).unwrap();
        let pointer_ty = op1.ty(body.locals()).unwrap();
        validate_offset(tcx, offset_ty, statement.span);
        validate_raw_ptr(tcx, pointer_ty, statement.span);
        tcx.dcx().abort_if_errors();

        let pointee_ty = pointer_ty.kind().builtin_deref(true).unwrap().ty;
        // The model takes the following parameters (PointeeType, PointerType, OffsetType).
        let model = self.models[&KaniModel::Offset];
        let params = vec![
            GenericArgKind::Type(pointee_ty),
            GenericArgKind::Type(pointer_ty),
            GenericArgKind::Type(offset_ty),
        ];
        let instance = Instance::resolve(model, &GenericArgs(params)).unwrap();
        body.insert_call(
            &instance,
            &mut source,
            InsertPosition::After,
            vec![op1.clone(), op2.clone()],
            place.clone(),
        );
        body.remove_stmt(bb, stmt);
        true
    }
}

struct ReplaceIntrinsicCallVisitor<'a, 'tcx> {
    models: &'a HashMap<KaniModel, FnDef>,
    locals: Vec<LocalDecl>,
    tcx: TyCtxt<'tcx>,
    changed: bool,
}

impl<'a, 'tcx> ReplaceIntrinsicCallVisitor<'a, 'tcx> {
    fn new(
        models: &'a HashMap<KaniModel, FnDef>,
        locals: Vec<LocalDecl>,
        tcx: TyCtxt<'tcx>,
    ) -> Self {
        ReplaceIntrinsicCallVisitor { models, locals, changed: false, tcx }
    }
}

impl MutMirVisitor for ReplaceIntrinsicCallVisitor<'_, '_> {
    /// Replace the terminator for some rustc's intrinsics.
    ///
    /// In some cases, we replace a function call to a rustc intrinsic by a call to the
    /// corresponding Kani intrinsic.
    ///
    /// Our models are usually augmented by some trait bounds, or they leverage Kani intrinsics to
    /// implement the given semantics.
    ///
    /// Note that we only need to replace function calls since intrinsics must always be called
    /// directly. I.e., no need to handle function pointers.
    fn visit_terminator(&mut self, term: &mut Terminator) {
        if let TerminatorKind::Call { func, .. } = &mut term.kind
            && let TyKind::RigidTy(RigidTy::FnDef(def, args)) =
                func.ty(&self.locals).unwrap().kind()
        {
            // Get the model we should use to replace this function call, if any.
            let replacement_model = if def.is_intrinsic() {
                let instance = Instance::resolve(def, &args).unwrap();
                let intrinsic = Intrinsic::from_instance(&instance);
                debug!(?intrinsic, "handle_terminator");
                match intrinsic {
                    Intrinsic::AlignOfVal => self.models[&KaniModel::AlignOfVal],
                    Intrinsic::SizeOfVal => self.models[&KaniModel::SizeOfVal],
                    Intrinsic::PtrOffsetFrom => self.models[&KaniModel::PtrOffsetFrom],
                    Intrinsic::PtrOffsetFromUnsigned => {
                        self.models[&KaniModel::PtrOffsetFromUnsigned]
                    }
                    // The rest is handled in codegen.
                    _ => {
                        return self.super_terminator(term);
                    }
                }
            } else if is_panic_function(&self.tcx, def.0) {
                // If we find a panic function, we replace it with our stub.
                self.models[&KaniModel::PanicStub]
            } else {
                return self.super_terminator(term);
            };

            let new_instance = Instance::resolve(replacement_model, &args).unwrap();

            // Construct the wrapper types needed to insert our resolved model [Instance]
            // back into the MIR as an operand.
            let literal = MirConst::try_new_zero_sized(new_instance.ty()).unwrap();
            let span = term.span;
            let new_func = ConstOperand { span, user_ty: None, const_: literal };
            *func = Operand::Constant(new_func);
            self.changed = true;
        }
        self.super_terminator(term);
    }
}

/// Validate whether the offset type is valid, i.e., `isize` or `usize`.
///
/// This will emit an error if the type is wrong but not abort.
/// Invoke `tcx.dcx().abort_if_errors()` to abort execution.
fn validate_offset(tcx: TyCtxt, offset_ty: Ty, span: Span) {
    if !matches!(
        offset_ty.kind(),
        TyKind::RigidTy(RigidTy::Int(IntTy::Isize)) | TyKind::RigidTy(RigidTy::Uint(UintTy::Usize))
    ) {
        tcx.dcx().span_err(
            rustc_internal::internal(tcx, span),
            format!("Expected `isize` or `usize` for offset type. Found `{offset_ty}` instead"),
        );
    }
}

/// Validate that we have a raw pointer otherwise emit an error.
///
/// This will emit an error if the type is wrong but not abort.
/// Invoke `tcx.dcx().abort_if_errors()` to abort execution.
fn validate_raw_ptr(tcx: TyCtxt, ptr_ty: Ty, span: Span) {
    let pointer_ty_kind = ptr_ty.kind();
    if !pointer_ty_kind.is_raw_ptr() {
        tcx.dcx().span_err(
            rustc_internal::internal(tcx, span),
            format!("Expected raw pointer for pointer type. Found `{ptr_ty}` instead"),
        );
    }
}
