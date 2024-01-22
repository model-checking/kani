// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Stable functions involving type manipulation.
//!
//! This may for now invoke functions that use internal Rust compiler APIs.
//! While we migrate to stable APIs, this module will contain stable versions of functions from
//! `typ.rs`.

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Type;
use rustc_middle::ty::layout::{LayoutOf, TyAndLayout};
use rustc_middle::ty::{self};
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{Local, Operand, Place, Rvalue};
use stable_mir::ty::{FnSig, RigidTy, Ty, TyKind};

impl<'tcx> GotocCtx<'tcx> {
    pub fn place_ty_stable(&self, place: &Place) -> Ty {
        place.ty(self.current_fn().locals()).unwrap()
    }

    pub fn codegen_ty_stable(&mut self, ty: Ty) -> Type {
        self.codegen_ty(rustc_internal::internal(self.tcx, ty))
    }

    pub fn codegen_ty_ref_stable(&mut self, ty: Ty) -> Type {
        self.codegen_ty_ref(rustc_internal::internal(self.tcx, ty))
    }

    pub fn local_ty_stable(&self, local: Local) -> Ty {
        self.current_fn().locals()[local].ty
    }

    pub fn operand_ty_stable(&self, operand: &Operand) -> Ty {
        operand.ty(self.current_fn().locals()).unwrap()
    }

    pub fn is_zst_stable(&self, ty: Ty) -> bool {
        self.is_zst(rustc_internal::internal(self.tcx, ty))
    }

    pub fn layout_of_stable(&self, ty: Ty) -> TyAndLayout<'tcx> {
        self.layout_of(rustc_internal::internal(self.tcx, ty))
    }

    pub fn codegen_fndef_type_stable(&mut self, instance: Instance) -> Type {
        let func = self.symbol_name_stable(instance);
        self.ensure_struct(
            format!("{func}::FnDefStruct"),
            format!("{}::FnDefStruct", instance.name()),
            |_, _| vec![],
        )
    }

    pub fn fn_sig_of_instance_stable(&self, instance: Instance) -> FnSig {
        let fn_sig = self.fn_sig_of_instance(rustc_internal::internal(self.tcx, instance));
        let fn_sig =
            self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), fn_sig);
        rustc_internal::stable(fn_sig)
    }

    pub fn use_fat_pointer_stable(&self, pointer_ty: Ty) -> bool {
        self.use_fat_pointer(rustc_internal::internal(self.tcx, pointer_ty))
    }

    pub fn use_thin_pointer_stable(&self, pointer_ty: Ty) -> bool {
        self.use_thin_pointer(rustc_internal::internal(self.tcx, pointer_ty))
    }

    pub fn is_fat_pointer_stable(&self, pointer_ty: Ty) -> bool {
        self.is_fat_pointer(rustc_internal::internal(self.tcx, pointer_ty))
    }

    pub fn is_vtable_fat_pointer_stable(&self, pointer_ty: Ty) -> bool {
        self.is_vtable_fat_pointer(rustc_internal::internal(self.tcx, pointer_ty))
    }

    pub fn use_vtable_fat_pointer_stable(&self, pointer_ty: Ty) -> bool {
        self.use_vtable_fat_pointer(rustc_internal::internal(self.tcx, pointer_ty))
    }

    pub fn vtable_name_stable(&self, ty: Ty) -> String {
        self.vtable_name(rustc_internal::internal(self.tcx, ty))
    }

    pub fn rvalue_ty_stable(&self, rvalue: &Rvalue) -> Ty {
        rvalue.ty(self.current_fn().locals()).unwrap()
    }

    pub fn simd_size_and_type(&self, ty: Ty) -> (u64, Ty) {
        let (sz, ty) = rustc_internal::internal(self.tcx, ty).simd_size_and_type(self.tcx);
        (sz, rustc_internal::stable(ty))
    }

    pub fn codegen_enum_discr_typ_stable(&self, ty: Ty) -> Ty {
        rustc_internal::stable(self.codegen_enum_discr_typ(rustc_internal::internal(self.tcx, ty)))
    }

    pub fn codegen_function_sig_stable(&mut self, sig: FnSig) -> Type {
        let params = sig
            .inputs()
            .iter()
            .filter_map(|ty| {
                if self.is_zst_stable(*ty) { None } else { Some(self.codegen_ty_stable(*ty)) }
            })
            .collect();

        if sig.c_variadic {
            Type::variadic_code_with_unnamed_parameters(
                params,
                self.codegen_ty_stable(sig.output()),
            )
        } else {
            Type::code_with_unnamed_parameters(params, self.codegen_ty_stable(sig.output()))
        }
    }

    /// Convert a type into a user readable type representation.
    ///
    /// This should be replaced by StableMIR `pretty_ty()` after
    /// <https://github.com/rust-lang/rust/pull/118364> is merged.
    pub fn pretty_ty(&self, ty: Ty) -> String {
        rustc_internal::internal(self.tcx, ty).to_string()
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

pub fn pointee_type_stable(ty: Ty) -> Option<Ty> {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Ref(_, pointee_ty, _))
        | TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, ..)) => Some(pointee_ty),
        _ => None,
    }
}
