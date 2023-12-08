// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Stable functions involving type manipulation.
//!
//! This may for now invoke functions that use internal Rust compiler APIs.
//! While we migrate to stable APIs, this module will contain stable versions of functions from
//! `typ.rs`.

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Type;
use rustc_middle::mir;
use rustc_middle::mir::visit::{MutVisitor, NonUseContext, PlaceContext};
use rustc_middle::mir::{
    Operand as OperandInternal, Place as PlaceInternal, Rvalue as RvalueInternal,
};
use rustc_middle::ty::{self, Const as ConstInternal, Ty as TyInternal, TyCtxt};
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{Local, Operand, Place, Rvalue};
use stable_mir::ty::{Const, RigidTy, Ty, TyKind};

impl<'tcx> GotocCtx<'tcx> {
    pub fn place_ty_stable(&self, place: &Place) -> Ty {
        place.ty(self.current_fn().body().locals()).unwrap()
    }

    pub fn codegen_ty_stable(&mut self, ty: Ty) -> Type {
        self.codegen_ty(rustc_internal::internal(ty))
    }

    pub fn local_ty_stable(&mut self, local: Local) -> Ty {
        self.current_fn().body().local_decl(local).unwrap().ty
    }

    pub fn operand_ty_stable(&mut self, operand: &Operand) -> Ty {
        operand.ty(self.current_fn().body().locals()).unwrap()
    }

    pub fn is_zst_stable(&self, ty: Ty) -> bool {
        self.is_zst(rustc_internal::internal(ty))
    }

    pub fn codegen_fndef_type_stable(&mut self, instance: Instance) -> Type {
        let func = self.symbol_name_stable(instance);
        self.ensure_struct(
            format!("{func}::FnDefStruct"),
            format!("{}::FnDefStruct", instance.name()),
            |_, _| vec![],
        )
    }

    pub fn fn_sig_of_instance_stable(&self, instance: Instance) -> ty::PolyFnSig<'tcx> {
        self.fn_sig_of_instance(rustc_internal::internal(instance))
    }

    pub fn use_fat_pointer_stable(&self, pointer_ty: Ty) -> bool {
        self.use_fat_pointer(rustc_internal::internal(pointer_ty))
    }

    pub fn is_fat_pointer_stable(&self, pointer_ty: Ty) -> bool {
        self.is_fat_pointer(rustc_internal::internal(pointer_ty))
    }

    pub fn is_vtable_fat_pointer_stable(&self, pointer_ty: Ty) -> bool {
        self.is_vtable_fat_pointer(rustc_internal::internal(pointer_ty))
    }

    pub fn vtable_name_stable(&self, ty: Ty) -> String {
        self.vtable_name(rustc_internal::internal(ty))
    }

    pub fn rvalue_ty_stable(&self, rvalue: &Rvalue) -> Ty {
        rvalue.ty(self.current_fn().body().locals()).unwrap()
    }
}
/// If given type is a Ref / Raw ref, return the pointee type.
pub fn pointee_type(mir_type: Ty) -> Option<Ty> {
    match mir_type.kind() {
        TyKind::RigidTy(RigidTy::Ref(_, pointee_type, _)) => Some(pointee_type),
        TyKind::RigidTy(RigidTy::RawPtr(ty, ..)) => Some(ty),
        _ => None,
    }
}

/// Convert internal rustc's structs into StableMIR ones.
///
/// The body of a StableMIR instance already comes monomorphized, which is different from rustc's
/// internal representation. To allow us to migrate parts of the code generation stage with
/// smaller PRs, we have to instantiate rustc's components when converting them to stable.
///
/// Once we finish migrating the entire function code generation, we can remove this code.
pub struct StableConverter<'a, 'tcx> {
    gcx: &'a GotocCtx<'tcx>,
}

impl<'a, 'tcx> StableConverter<'a, 'tcx> {
    pub fn convert_place(gcx: &'a GotocCtx<'tcx>, mut place: PlaceInternal<'tcx>) -> Place {
        let mut converter = StableConverter { gcx };
        converter.visit_place(
            &mut place,
            PlaceContext::NonUse(NonUseContext::VarDebugInfo),
            mir::Location::START,
        );
        rustc_internal::stable(place)
    }

    pub fn convert_rvalue(gcx: &'a GotocCtx<'tcx>, mut operand: RvalueInternal<'tcx>) -> Rvalue {
        let mut converter = StableConverter { gcx };
        converter.visit_rvalue(&mut operand, mir::Location::START);
        rustc_internal::stable(operand)
    }

    pub fn convert_operand(gcx: &'a GotocCtx<'tcx>, mut operand: OperandInternal<'tcx>) -> Operand {
        let mut converter = StableConverter { gcx };
        converter.visit_operand(&mut operand, mir::Location::START);
        rustc_internal::stable(operand)
    }

    pub fn convert_constant(gcx: &'a GotocCtx<'tcx>, mut constant: ConstInternal<'tcx>) -> Const {
        let mut converter = StableConverter { gcx };
        converter.visit_ty_const(&mut constant, mir::Location::START);
        rustc_internal::stable(constant)
    }
}

pub fn pointee_type_stable(ty: Ty) -> Option<Ty> {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Ref(_, pointee_type, _))
        | TyKind::RigidTy(RigidTy::RawPtr(ty, ..)) => Some(pointee_type),
        _ => None,
    }
}

impl<'a, 'tcx> MutVisitor<'tcx> for StableConverter<'a, 'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.gcx.tcx
    }

    fn visit_ty(&mut self, ty: &mut TyInternal<'tcx>, _: mir::visit::TyContext) {
        *ty = self.gcx.monomorphize(*ty);
    }

    fn visit_ty_const(&mut self, ct: &mut ty::Const<'tcx>, _location: mir::Location) {
        *ct = self.gcx.monomorphize(*ct);
    }

    fn visit_constant(&mut self, constant: &mut mir::ConstOperand<'tcx>, location: mir::Location) {
        let const_ = self.gcx.monomorphize(constant.const_);
        let val = match const_.eval(self.gcx.tcx, ty::ParamEnv::reveal_all(), None) {
            Ok(v) => v,
            Err(mir::interpret::ErrorHandled::Reported(..)) => return,
            Err(mir::interpret::ErrorHandled::TooGeneric(..)) => {
                unreachable!("Failed to evaluate instance constant: {:?}", const_)
            }
        };
        let ty = constant.ty();
        constant.const_ = mir::Const::Val(val, ty);
        self.super_constant(constant, location);
    }
}
