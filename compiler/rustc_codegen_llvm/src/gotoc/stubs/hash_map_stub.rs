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

pub struct HashMapStub<'tcx> {
    ty_opt: Cell<Option<Option<Ty<'tcx>>>>,
    stubbed_fns: RefCell<FxHashMap<String, Option<DefId>>>,
}

impl<'tcx> RustStubber<'tcx> for HashMapStub<'tcx> {
    fn get_old_type_prefix(&self) -> &'static str {
        "std::collections::HashMap<"
    }
    fn get_new_type_name(&self) -> &'static str {
        "CbmcHashMap"
    }
    fn get_ty_opt_field(&self) -> &Cell<Option<Option<Ty<'tcx>>>> {
        &self.ty_opt
    }
    fn get_cbmc_fn_stub_table(&self) -> RefMut<'_, FxHashMap<String, Option<DefId>>> {
        self.stubbed_fns.borrow_mut()
    }
}

impl<'tcx> HashMapStub<'tcx> {
    pub fn new() -> HashMapStub<'tcx> {
        Self { ty_opt: Cell::new(None), stubbed_fns: RefCell::new(FxHashMap::default()) }
    }
}

impl<'tcx> GotocHook<'tcx> for HashMapStub<'tcx> {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        if !self.stub_type_was_defined(tcx) {
            return false;
        }

        let is_destructor = self.is_target_destructor(instance);
        let unmangled = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        println!("*** Unmangled was {} {} ", unmangled, is_destructor);
        let matched = match &unmangled[..] {
            "std::collections::HashMap::<K, V>::new" => true,
            "std::collections::HashMap::<K, V, S>::insert" => true,
            "std::collections::HashMap::<K, V, S>::get" => true,
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
            "std::collections::HashMap::<K, V>::new" => {
                self.translate_to_stub(tcx, instance, fargs, assign_to, target, "new")
            }
            "std::collections::HashMap::<K, V, S>::insert" => {
                self.translate_to_stub(tcx, instance, fargs, assign_to, target, "insert")
            }
            "std::collections::HashMap::<K, V, S>::get" => {
                self.translate_to_stub(tcx, instance, fargs, assign_to, target, "get")
            }
            _ if self.is_target_destructor(instance) => {
                Stmt::goto(tcx.current_fn().find_label(&target.unwrap()), Location::none())
            }
            _ => unreachable!(),
        }
    }
}
