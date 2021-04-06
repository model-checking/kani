// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module contains various codegen hooks for functions.
//! e.g.
//! functions start with [__nondet] is silently replaced by nondeterministic values, and
//! [begin_panic] is replaced by [assert(false)], etc.
//!
//! It would be too nasty if we spread around these sort of undocumented hooks in place, so
//! this module addresses this issue.

use super::cbmc::goto_program::{BuiltinFn, Expr, Location, Stmt, Symbol, Type};
use super::metadata::GotocCtx;
use super::stubs::hash_map_stub::HashMapStub;
use super::stubs::vec_stub::VecStub;
use rustc_hir::definitions::DefPathDataName;
use rustc_middle::mir::{BasicBlock, Place};
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::{self, Instance, InstanceDef, Ty, TyCtxt};
use rustc_span::Span;
use rustc_target::abi::LayoutOf;
use std::rc::Rc;
pub trait GotocTypeHook<'tcx> {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool;
    fn handle(&self, tcx: &mut GotocCtx<'tcx>, ty: Ty<'tcx>) -> Type;
}

pub trait GotocHook<'tcx> {
    /// if the hook applies, it means the codegen would do something special to it
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool;
    /// the handler for codegen
    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt;
}

fn sig_of_instance<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> ty::FnSig<'tcx> {
    let ty = instance.ty(tcx, ty::ParamEnv::reveal_all());
    let sig = match ty.kind() {
        ty::Closure(_, substs) => substs.as_closure().sig(),
        _ => ty.fn_sig(tcx),
    };
    tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig)
}

struct Nondet;

impl<'tcx> GotocHook<'tcx> for Nondet {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let sig = sig_of_instance(tcx, instance);
        let def_path = tcx.def_path(instance.def.def_id());
        if let Some(data) = def_path.data.last() {
            match data.data.name() {
                DefPathDataName::Named(name) => {
                    return name.to_string().starts_with("__nondet")
                        && sig.inputs().is_empty()
                        && !sig.output().is_never();
                }
                DefPathDataName::Anon { .. } => (),
            }
        }
        false
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        _fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        match (assign_to, target) {
            (Some(p), Some(target)) => {
                let pt = tcx.place_ty(&p);
                if pt.is_unit() {
                    Stmt::goto(tcx.find_label(&target), Location::none())
                } else {
                    let pe = tcx.codegen_place(&p).goto_expr;
                    Stmt::block(vec![
                        pe.clone().assign(tcx.codegen_ty(pt).nondet()),
                        // we should potentially generate an assumption
                        match tcx.codegen_assumption(pt) {
                            None => Stmt::skip(Location::none()),
                            Some(f) => {
                                Stmt::assume(f.call(vec![pe.address_of()]), Location::none())
                            }
                        },
                        Stmt::goto(tcx.find_label(&target), Location::none()),
                    ])
                }
            }
            _ => unreachable!(),
        }
    }
}

struct Panic;

impl<'tcx> GotocHook<'tcx> for Panic {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let sig = sig_of_instance(tcx, instance);
        let def_path = tcx.def_path(instance.def.def_id());
        if let Some(data) = def_path.data.last() {
            match data.data.name() {
                DefPathDataName::Named(name) => {
                    return (name.to_string() == "begin_panic" || name.to_string() == "panic")
                        && sig.output().is_never();
                }
                DefPathDataName::Anon { .. } => (),
            }
        }
        false
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        _target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        tcx.codegen_panic(span, fargs)
    }
}

struct Nevers;

impl<'tcx> GotocHook<'tcx> for Nevers {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let sig = sig_of_instance(tcx, instance);
        sig.output().is_never()
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        _fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        _target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option2(span);
        // _target must be None due to how rust compiler considers it
        Stmt::assert_false(
            &format!(
                "a panicking function {} is invoked",
                with_no_trimmed_paths(|| tcx.tcx.def_path_str(instance.def_id()))
            ),
            loc,
        )
    }
}

struct Intrinsic;

impl<'tcx> GotocHook<'tcx> for Intrinsic {
    fn hook_applies(&self, _tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        match instance.def {
            InstanceDef::Intrinsic(_) => true,
            _ => false,
        }
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        match (assign_to, target) {
            (Some(p), Some(target)) => Stmt::block(vec![
                tcx.codegen_intrinsic(instance, fargs, &p, span),
                Stmt::goto(tcx.find_label(&target), Location::none()),
            ]),
            _ => unreachable!(),
        }
    }
}

struct MemReplace;

impl<'tcx> GotocHook<'tcx> for MemReplace {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::mem::replace" || name == "std::mem::replace"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        match (assign_to, target) {
            (Some(p), Some(target)) => {
                // Skip an assignment to a destination that has a zero-sized type
                // (For a ZST, Rust optimizes away the source and fargs.len() == 1)
                let place_type = tcx.place_ty(&p);
                let place_layout = tcx.layout_of(place_type);
                let place_is_zst = place_layout.is_zst();
                if place_is_zst {
                    Stmt::block(vec![Stmt::goto(tcx.find_label(&target), Location::none())])
                } else {
                    let dest = fargs.remove(0);
                    let src = fargs.remove(0);
                    Stmt::block(vec![
                        tcx.codegen_place(&p).goto_expr.assign(dest.clone().dereference()),
                        dest.dereference().assign(src),
                        Stmt::goto(tcx.find_label(&target), Location::none()),
                    ])
                }
            }
            _ => unreachable!(),
        }
    }
}

struct MemSwap;

impl<'tcx> GotocHook<'tcx> for MemSwap {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::mem::swap"
            || name == "std::mem::swap"
            || name == "core::ptr::swap"
            || name == "std::ptr::swap"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        let ty = tcx.monomorphize(instance.substs.type_at(0));
        match (assign_to, target) {
            (Some(_), Some(target)) => {
                let x = fargs.remove(0);
                let y = fargs.remove(0);

                let func_name = format!("gen-swap<{}>", tcx.ty_mangled_name(ty));
                tcx.ensure(&func_name, |tcx, _| {
                    let ty = tcx.codegen_ty(ty);
                    let x_param =
                        tcx.gen_function_local_variable(1, &func_name, ty.clone().to_pointer());
                    let y_param =
                        tcx.gen_function_local_variable(2, &func_name, ty.clone().to_pointer());
                    let var = tcx.gen_function_local_variable(3, &func_name, ty);
                    let mut block = Vec::new();
                    let xe = x_param.to_expr();
                    block.push(Stmt::decl(
                        var.to_expr(),
                        Some(xe.clone().dereference()),
                        Location::none(),
                    ));
                    let ye = y_param.to_expr();
                    let var = var.to_expr();
                    block.push(xe.dereference().assign(ye.clone().dereference()));
                    block.push(ye.dereference().assign(var));

                    Symbol::function(
                        &func_name,
                        Type::code(
                            vec![x_param.to_function_parameter(), y_param.to_function_parameter()],
                            Type::empty(),
                        ),
                        Some(Stmt::block(block)),
                        Location::none(),
                    )
                });

                Stmt::block(vec![
                    tcx.find_function(&func_name).unwrap().call(vec![x, y]).as_stmt(),
                    Stmt::goto(tcx.find_label(&target), Location::none()),
                ])
            }
            _ => unreachable!(),
        }
    }
}

struct PtrRead;

impl<'tcx> GotocHook<'tcx> for PtrRead {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::ptr::read"
            || name == "core::ptr::read_unaligned"
            || name == "core::ptr::read_volatile"
            || name == "std::ptr::read"
            || name == "std::ptr::read_unaligned"
            || name == "std::ptr::read_volatile"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        match (assign_to, target) {
            (Some(p), Some(target)) => {
                let src = fargs.remove(0);
                Stmt::block(vec![
                    tcx.codegen_place(&p).goto_expr.assign(src.dereference()),
                    Stmt::goto(tcx.find_label(&target), Location::none()),
                ])
            }
            _ => unreachable!(),
        }
    }
}

struct PtrWrite;

impl<'tcx> GotocHook<'tcx> for PtrWrite {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::ptr::write"
            || name == "core::ptr::write_unaligned"
            || name == "core::ptr::write_volatile"
            || name == "std::ptr::write"
            || name == "std::ptr::write_unaligned"
            || name == "std::ptr::write_volatile"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        match target {
            Some(target) => {
                let dst = fargs.remove(0);
                let src = fargs.remove(0);
                Stmt::block(vec![
                    dst.dereference().assign(src),
                    Stmt::goto(tcx.find_label(&target), Location::none()),
                ])
            }
            _ => unreachable!(),
        }
    }
}

struct RustAlloc;

impl<'tcx> GotocHook<'tcx> for RustAlloc {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = tcx.symbol_name(instance).name.to_string();
        name == "__rust_alloc"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option2(span);
        match (assign_to, target) {
            (Some(p), Some(target)) => {
                let size = fargs.remove(0);
                Stmt::block(vec![
                    tcx.codegen_place(&p).goto_expr.assign(
                        BuiltinFn::Malloc
                            .call(vec![size], loc)
                            .cast_to(Type::unsigned_int(8).to_pointer()),
                    ),
                    Stmt::goto(tcx.find_label(&target), Location::none()),
                ])
            }
            _ => unreachable!(),
        }
    }
}

struct RustDealloc;

impl<'tcx> GotocHook<'tcx> for RustDealloc {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = tcx.symbol_name(instance).name.to_string();
        name == "__rust_dealloc"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option2(span);
        match target {
            Some(target) => {
                let ptr = fargs.remove(0);
                Stmt::block(vec![
                    BuiltinFn::Free.call(vec![ptr.cast_to(Type::void_pointer())], loc).as_stmt(),
                    Stmt::goto(tcx.find_label(&target), Location::none()),
                ])
            }
            _ => unreachable!(),
        }
    }
}

struct RustRealloc;

impl<'tcx> GotocHook<'tcx> for RustRealloc {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = tcx.symbol_name(instance).name.to_string();
        name == "__rust_realloc"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option2(span);
        match (assign_to, target) {
            (Some(p), Some(target)) => {
                let ptr = fargs.remove(0).cast_to(Type::void_pointer());
                fargs.remove(0); // old_size
                fargs.remove(0); // align
                let size = fargs.remove(0);
                Stmt::block(vec![
                    tcx.codegen_place(&p).goto_expr.assign(
                        BuiltinFn::Realloc
                            .call(vec![ptr, size], loc)
                            .cast_to(Type::unsigned_int(8).to_pointer()),
                    ),
                    Stmt::goto(tcx.find_label(&target), Location::none()),
                ])
            }
            _ => unreachable!(),
        }
    }
}

struct RustAllocZeroed;

impl<'tcx> GotocHook<'tcx> for RustAllocZeroed {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = tcx.symbol_name(instance).name.to_string();
        name == "__rust_alloc_zeroed"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option2(span);
        match (assign_to, target) {
            (Some(p), Some(target)) => {
                let size = fargs.remove(0);
                Stmt::block(vec![
                    tcx.codegen_place(&p).goto_expr.assign(
                        BuiltinFn::Calloc
                            .call(vec![Type::size_t().one(), size], loc)
                            .cast_to(Type::unsigned_int(8).to_pointer()),
                    ),
                    Stmt::goto(tcx.find_label(&target), Location::none()),
                ])
            }
            _ => unreachable!(),
        }
    }
}

struct SliceFromRawPart;

impl<'tcx> GotocHook<'tcx> for SliceFromRawPart {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::ptr::slice_from_raw_parts"
            || name == "std::ptr::slice_from_raw_parts"
            || name == "core::ptr::slice_from_raw_parts_mut"
            || name == "std::ptr::slice_from_raw_parts_mut"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        match (assign_to, target) {
            (Some(p), Some(target)) => {
                let pt = tcx.codegen_ty(tcx.place_ty(&p));
                let data = fargs.remove(0);
                let len = fargs.remove(0);
                let code = tcx
                    .codegen_place(&p)
                    .goto_expr
                    .assign(Expr::struct_expr_from_values(pt, vec![data, len], &tcx.symbol_table))
                    .with_location(tcx.codegen_span_option2(span));
                Stmt::block(vec![code, Stmt::goto(tcx.find_label(&target), Location::none())])
            }
            _ => unreachable!(),
        }
    }
}

pub fn type_and_fn_hooks<'tcx>() -> (GotocTypeHooks<'tcx>, GotocHooks<'tcx>) {
    let vec_stub = Rc::new(VecStub::new());
    let hash_map_stub = Rc::new(HashMapStub::new());
    let thks = GotocTypeHooks { hooks: vec![hash_map_stub.clone(), vec_stub.clone()] };
    let fhks = GotocHooks {
        hooks: vec![
            Rc::new(Panic), //Must go first, so it overrides Nevers
            Rc::new(Nevers),
            Rc::new(Nondet),
            Rc::new(MemReplace),
            Rc::new(MemSwap),
            Rc::new(PtrRead),
            Rc::new(PtrWrite),
            Rc::new(RustAlloc),
            Rc::new(RustDealloc),
            Rc::new(RustRealloc),
            Rc::new(RustAllocZeroed),
            Rc::new(SliceFromRawPart),
            Rc::new(Intrinsic),
            hash_map_stub.clone(),
            vec_stub.clone(),
        ],
    };
    (thks, fhks)
}

pub struct GotocTypeHooks<'tcx> {
    hooks: Vec<Rc<dyn GotocTypeHook<'tcx> + 'tcx>>,
}

impl<'tcx> GotocTypeHooks<'tcx> {
    pub fn default() -> GotocTypeHooks<'tcx> {
        type_and_fn_hooks().0
    }

    pub fn hook_applies(
        &self,
        tcx: TyCtxt<'tcx>,
        ty: Ty<'tcx>,
    ) -> Option<Rc<dyn GotocTypeHook<'tcx> + 'tcx>> {
        for h in &self.hooks {
            if h.hook_applies(tcx, ty) {
                return Some(h.clone());
            }
        }
        None
    }
}

pub struct GotocHooks<'tcx> {
    hooks: Vec<Rc<dyn GotocHook<'tcx> + 'tcx>>,
}

impl<'tcx> GotocHooks<'tcx> {
    pub fn default() -> GotocHooks<'tcx> {
        type_and_fn_hooks().1
    }

    pub fn hook_applies(
        &self,
        tcx: TyCtxt<'tcx>,
        instance: Instance<'tcx>,
    ) -> Option<Rc<dyn GotocHook<'tcx> + 'tcx>> {
        for h in &self.hooks {
            if h.hook_applies(tcx, instance) {
                return Some(h.clone());
            }
        }
        None
    }
}
