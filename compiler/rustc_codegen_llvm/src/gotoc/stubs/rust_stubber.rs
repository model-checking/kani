// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::super::cbmc::goto_program::{Expr, Location, Stmt, Type};
use super::super::hooks::GotocTypeHook;
use super::super::metadata::GotocCtx;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::definitions::{DefPathData, DisambiguatedDefPathData};
use rustc_hir::itemlikevisit::ItemLikeVisitor;
use rustc_hir::{ForeignItem, ImplItem, Item, ItemKind, TraitItem};
use rustc_middle::mir::{BasicBlock, Place};
use rustc_middle::ty::subst::Subst;
use rustc_middle::ty::{self, Instance, InstanceDef, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use std::cell::{Cell, RefMut};
pub struct CbmcTypeVisitor<'tcx> {
    cbmc_type_name: String,
    tcx: TyCtxt<'tcx>,
    pub ty_opt: Option<Ty<'tcx>>,
}

impl<'tcx> CbmcTypeVisitor<'tcx> {
    pub fn new(cbmc_type_name: String, tcx: TyCtxt<'tcx>) -> CbmcTypeVisitor<'_> {
        CbmcTypeVisitor { cbmc_type_name, tcx, ty_opt: None }
    }
}

impl<'tcx> ItemLikeVisitor<'tcx> for CbmcTypeVisitor<'tcx> {
    // Foreign items are items declared in "extern" blocks.
    fn visit_foreign_item(&mut self, item: &'hir ForeignItem<'hir>) {
        if item.ident.name.to_string() == self.cbmc_type_name {
            let vec_ty = self.tcx.type_of(self.tcx.hir().local_def_id(item.hir_id()).to_def_id());
            self.ty_opt = Some(vec_ty);
        }
    }

    fn visit_item(&mut self, item: &'tcx Item<'tcx>) {
        match item.kind {
            //TODO, I'm not sure this list is exhaustive.
            ItemKind::Enum(_, _) | ItemKind::Struct(_, _) | ItemKind::Union(_, _) => {
                if item.ident.name.to_string() == self.cbmc_type_name {
                    let vec_ty =
                        self.tcx.type_of(self.tcx.hir().local_def_id(item.hir_id()).to_def_id());
                    self.ty_opt = Some(vec_ty);
                }
            }
            _ => {}
        }
    }

    fn visit_trait_item(&mut self, _trait_item: &'tcx TraitItem<'tcx>) {}

    fn visit_impl_item(&mut self, _impl_item: &'tcx ImplItem<'tcx>) {}
}

pub trait RustStubber<'tcx> {
    fn stub_type_was_defined(&self, tcx: TyCtxt<'tcx>) -> bool {
        self.ty_opt(tcx).is_some()
    }

    fn ty_opt(&self, tcx: TyCtxt<'tcx>) -> Option<Ty<'tcx>> {
        if self.get_ty_opt_field().get().is_none() {
            let mut visitor = CbmcTypeVisitor::new(self.get_new_type_name().to_string(), tcx);
            tcx.hir().krate().visit_all_item_likes(&mut visitor);
            self.get_ty_opt_field().set(Some(visitor.ty_opt.take()));
        }
        // here ty_opt must be Some, so we just unwrap
        self.get_ty_opt_field().get().unwrap()
    }

    fn is_our_type(&self, ty: Ty<'tcx>) -> bool {
        format!("{:?}", ty).starts_with(self.get_old_type_prefix())
    }

    fn get_old_type_prefix(&self) -> &'static str;
    fn get_new_type_name(&self) -> &'static str;
    fn get_ty_opt_field(&self) -> &Cell<Option<Option<Ty<'tcx>>>>;
    fn get_cbmc_fn_stub_table(&self) -> RefMut<'_, FxHashMap<String, Option<DefId>>>;

    fn translate_to_stub(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        fn_name: &str,
    ) -> Stmt {
        let stubbed_fn = self
            .stub_fn(tcx.tcx, fn_name)
            .expect(&format!("Couldn't find the stub for function: {}", fn_name));
        self.translate_function(tcx, instance, fargs, assign_to, target, stubbed_fn)
    }

    fn stub_fn(&self, tcx: TyCtxt<'tcx>, fn_name: &str) -> Option<DefId> {
        let mut hash_table = self.get_cbmc_fn_stub_table();
        let cbmc_fn = hash_table.entry(fn_name.to_string()).or_insert(match self.ty_opt(tcx) {
            None => None,
            Some(t) => self.find_cbmc_fn(tcx, t, fn_name),
        });
        *cbmc_fn
    }

    fn translate_function(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        new_fn_def_id: DefId,
    ) -> Stmt {
        let new_fn_instance =
            Instance::resolve(tcx.tcx, ty::ParamEnv::reveal_all(), new_fn_def_id, instance.substs)
                .unwrap()
                .unwrap();

        let new_name = tcx.symbol_name(new_fn_instance);
        println!("translate_function: {} {:?}", new_name, new_fn_instance);
        let fctn = tcx.find_function(&new_name).unwrap();
        let p = assign_to.unwrap();
        let fn_call = fctn.call(fargs);
        let target = target.unwrap();
        Stmt::block(
            vec![
                tcx.codegen_expr_to_place(&p, fn_call),
                Stmt::goto(tcx.find_label(&target), Location::none()),
            ],
            Location::none(),
        )
    }

    fn find_cbmc_fn(&self, tcx: TyCtxt<'tcx>, stubbed_ty: Ty<'tcx>, name: &str) -> Option<DefId> {
        for def_id in tcx.mir_keys(()).iter().map(|def_id| def_id.to_def_id()) {
            let defpath = tcx.def_path(def_id);
            println!("defpath is {:?}, name is {}", defpath, name);
            match &defpath.data[..] {
                [.., DisambiguatedDefPathData { data: DefPathData::Impl, .. }, dpdata] => {
                    let key = tcx.def_key(def_id);
                    let impl_def_id = DefId { index: key.parent.unwrap(), ..def_id };
                    let self_ty = tcx.type_of(impl_def_id);
                    println!("self_ty is {:?}", self_ty);
                    let subst = match self_ty.kind() {
                        ty::Adt(_, substs) => substs,
                        _ => unreachable!(),
                    };
                    let stubbed_ty = stubbed_ty.subst(tcx, subst);
                    if self_ty == stubbed_ty && dpdata.data.to_string() == name {
                        return Some(def_id);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn is_target_destructor(&self, instance: Instance<'tcx>) -> bool {
        if let InstanceDef::DropGlue(_, Some(dest_ty)) = instance.def {
            self.is_our_type(dest_ty)
        } else {
            false
        }
    }
}

impl<'tcx, T> GotocTypeHook<'tcx> for T
where
    T: RustStubber<'tcx>,
{
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        self.stub_type_was_defined(tcx) && self.is_our_type(ty)
    }

    fn handle(&self, tcx: &mut GotocCtx<'tcx>, ty: Ty<'tcx>) -> Type {
        let new_type = self.ty_opt(tcx.tcx).unwrap();
        let subst = match ty.kind() {
            ty::Adt(_, substs) => substs,
            _ => unreachable!(),
        };
        let new_type_subst = new_type.subst(tcx.tcx, subst);
        tcx.codegen_ty(new_type_subst)
    }
}
