// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Stable functions involving type manipulation.
//!
//! This may for now invoke functions that use internal Rust compiler APIs.
//! While we migrate to stable APIs, this module will contain stable versions of functions from
//! `typ.rs`.

use crate::codegen_cprover_gotoc::GotocCtx;
use stable_mir::mir::Place;
use stable_mir::ty::Ty;

impl<'tcx> GotocCtx<'tcx> {
    pub fn place_ty_stable(&self, place: &Place) -> Ty {
        place.ty(self.current_fn().body().locals()).unwrap()
    }
}
