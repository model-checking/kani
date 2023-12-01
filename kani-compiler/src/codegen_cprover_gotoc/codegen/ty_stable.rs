// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Stable functions involving type manipulation.
//!
//! This may for now invoke functions that use internal Rust compiler APIs.
//! While we migrate to stable APIs, this module will contain stable versions of functions from
//! `typ.rs`.

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Type;
use rustc_smir::rustc_internal;
use stable_mir::mir::{Local, Place};
use stable_mir::ty::{RigidTy, Ty, TyKind};

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
}
/// If given type is a Ref / Raw ref, return the pointee type.
pub fn pointee_type(mir_type: Ty) -> Option<Ty> {
    match mir_type.kind() {
        TyKind::RigidTy(RigidTy::Ref(_, pointee_type, _)) => Some(pointee_type),
        TyKind::RigidTy(RigidTy::RawPtr(ty, ..)) => Some(ty),
        _ => None,
    }
}
