// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::rust_stubber::RustStubber;
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::mir::{BasicBlock, Place};
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::{Instance, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use rustc_span::Span;
use std::cell::{Cell, RefCell, RefMut};

use super::super::cbmc::goto_program::{Expr, Location, Stmt};
use super::super::hooks::GotocHook;
use super::super::metadata::GotocCtx;

pub struct VecStub<'tcx> {
    ty_opt: Cell<Option<Option<Ty<'tcx>>>>,
    stubbed_fns: RefCell<FxHashMap<String, Option<DefId>>>,
}

impl<'tcx> RustStubber<'tcx> for VecStub<'tcx> {
    fn get_old_type_prefix(&self) -> &'static str {
        "std::vec::Vec<"
    }
    fn get_new_type_name(&self) -> &'static str {
        "CbmcVec"
    }
    fn get_ty_opt_field(&self) -> &Cell<Option<Option<Ty<'tcx>>>> {
        &self.ty_opt
    }
    fn get_cbmc_fn_stub_table(&self) -> RefMut<'_, FxHashMap<String, Option<DefId>>> {
        self.stubbed_fns.borrow_mut()
    }
}

impl<'tcx> VecStub<'tcx> {
    pub fn new() -> VecStub<'tcx> {
        Self { ty_opt: Cell::new(None), stubbed_fns: RefCell::new(FxHashMap::default()) }
    }
}

impl<'tcx> GotocHook<'tcx> for VecStub<'tcx> {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        if !self.stub_type_was_defined(tcx) {
            return false;
        }

        let is_destructor = self.is_target_destructor(instance);
        let unmangled = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        let matched = match &unmangled[..] {
            "std::vec::Vec::<T, A>::push"
            | "std::vec::Vec::<T>::new"
            | "std::vec::Vec::<T, A>::pop"
            | "std::vec::Vec::<T, A>::len" => true,
            _ if is_destructor => true,
            _ => false,
        };
        matched
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        let old_unmangled = with_no_trimmed_paths(|| tcx.tcx.def_path_str(instance.def_id()));
        println!("Handeling {}", old_unmangled);

        match &old_unmangled[..] {
            "std::vec::Vec::<T, A>::pop" => {
                self.translate_to_stub(tcx, instance, fargs, assign_to, target, "pop")
            }
            "std::vec::Vec::<T, A>::len" => {
                self.translate_to_stub(tcx, instance, fargs, assign_to, target, "len")
            }
            "std::vec::Vec::<T, A>::push" => {
                self.translate_to_stub(tcx, instance, fargs, assign_to, target, "push")
            }
            "std::vec::Vec::<T>::new" => {
                self.translate_to_stub(tcx, instance, fargs, assign_to, target, "new")
            }
            _ if self.is_target_destructor(instance) => {
                Stmt::goto(tcx.find_label(&target.unwrap()), Location::none())
            }
            _ => unreachable!(),
        }
    }
}
