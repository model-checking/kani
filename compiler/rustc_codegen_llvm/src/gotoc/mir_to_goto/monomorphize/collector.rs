// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::gotoc::mir_to_goto::overrides::GotocHooks;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_data_structures::sync::{par_iter, MTLock, MTRef, ParallelIterator};
use rustc_errors::{ErrorReported, FatalError};
use rustc_hir as hir;
use rustc_hir::def_id::{DefId, DefIdMap, LocalDefId};
use rustc_hir::itemlikevisit::ItemLikeVisitor;
use rustc_hir::lang_items::LangItem;
use rustc_index::bit_set::GrowableBitSet;
use rustc_middle::middle::codegen_fn_attrs::CodegenFnAttrFlags;
use rustc_middle::mir::interpret::{AllocId, ConstValue};
use rustc_middle::mir::interpret::{ErrorHandled, GlobalAlloc, Scalar};
use rustc_middle::mir::mono::{InstantiationMode, MonoItem};
use rustc_middle::mir::visit::Visitor as MirVisitor;
use rustc_middle::mir::{self, Local, Location};
use rustc_middle::traits;
use rustc_middle::ty::adjustment::{CustomCoerceUnsized, PointerCast};
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::subst::{GenericArgKind, InternalSubsts};
use rustc_middle::ty::{self, GenericParamDefKind, Instance, Ty, TyCtxt, TypeFoldable, VtblEntry};
use rustc_session::config::EntryFnType;
use rustc_span::source_map::{dummy_spanned, respan, Span, Spanned, DUMMY_SP};
use smallvec::SmallVec;
use std::iter;
use tracing::{debug, trace};

pub fn custom_coerce_unsize_info<'tcx>(
    tcx: TyCtxt<'tcx>,
    source_ty: Ty<'tcx>,
    target_ty: Ty<'tcx>,
) -> CustomCoerceUnsized {
    let def_id = tcx.require_lang_item(LangItem::CoerceUnsized, None);

    let trait_ref = ty::Binder::dummy(ty::TraitRef {
        def_id,
        substs: tcx.mk_substs_trait(source_ty, &[target_ty.into()]),
    });

    match tcx.codegen_fulfill_obligation((ty::ParamEnv::reveal_all(), trait_ref)) {
        Ok(traits::ImplSource::UserDefined(traits::ImplSourceUserDefinedData {
            impl_def_id,
            ..
        })) => tcx.coerce_unsized_info(impl_def_id).custom_kind.unwrap(),
        impl_source => {
            unreachable!("invalid `CoerceUnsized` impl_source: {:?}", impl_source);
        }
    }
}

#[derive(PartialEq)]
pub enum MonoItemCollectionMode {
    Eager,
    Lazy,
}

/// Maps every mono item to all mono items it references in its
/// body.
pub struct InliningMap<'tcx> {
    // Maps a source mono item to the range of mono items
    // accessed by it.
    // The two numbers in the tuple are the start (inclusive) and
    // end index (exclusive) within the `targets` vecs.
    index: FxHashMap<MonoItem<'tcx>, (usize, usize)>,
    targets: Vec<MonoItem<'tcx>>,

    // Contains one bit per mono item in the `targets` field. That bit
    // is true if that mono item needs to be inlined into every CGU.
    inlines: GrowableBitSet<usize>,
}

impl<'tcx> InliningMap<'tcx> {
    fn new() -> InliningMap<'tcx> {
        InliningMap {
            index: FxHashMap::default(),
            targets: Vec::new(),
            inlines: GrowableBitSet::with_capacity(1024),
        }
    }

    fn record_accesses(&mut self, source: MonoItem<'tcx>, new_targets: &[(MonoItem<'tcx>, bool)]) {
        let start_index = self.targets.len();
        let new_items_count = new_targets.len();
        let new_items_count_total = new_items_count + self.targets.len();

        self.targets.reserve(new_items_count);
        self.inlines.ensure(new_items_count_total);

        for (i, (target, inline)) in new_targets.iter().enumerate() {
            self.targets.push(*target);
            if *inline {
                self.inlines.insert(i + start_index);
            }
        }

        let end_index = self.targets.len();
        assert!(self.index.insert(source, (start_index, end_index)).is_none());
    }

    // Internally iterate over all items referenced by `source` which will be
    // made available for inlining.
    pub fn with_inlining_candidates<F>(&self, source: MonoItem<'tcx>, mut f: F)
    where
        F: FnMut(MonoItem<'tcx>),
    {
        if let Some(&(start_index, end_index)) = self.index.get(&source) {
            for (i, candidate) in self.targets[start_index..end_index].iter().enumerate() {
                if self.inlines.contains(start_index + i) {
                    f(*candidate);
                }
            }
        }
    }

    // Internally iterate over all items and the things each accesses.
    pub fn iter_accesses<F>(&self, mut f: F)
    where
        F: FnMut(MonoItem<'tcx>, &[MonoItem<'tcx>]),
    {
        for (&accessor, &(start_index, end_index)) in &self.index {
            f(accessor, &self.targets[start_index..end_index])
        }
    }
}

pub fn collect_crate_mono_items(
    tcx: TyCtxt<'_>,
    mode: MonoItemCollectionMode,
) -> (FxHashSet<MonoItem<'_>>, InliningMap<'_>) {
    let _prof_timer = tcx.prof.generic_activity("monomorphization_collector");

    let roots =
        tcx.sess.time("monomorphization_collector_root_collections", || collect_roots(tcx, mode));

    debug!("building mono item graph, beginning at roots");

    let mut visited = MTLock::new(FxHashSet::default());
    let mut inlining_map = MTLock::new(InliningMap::new());

    {
        let visited: MTRef<'_, _> = &mut visited;
        let inlining_map: MTRef<'_, _> = &mut inlining_map;

        tcx.sess.time("monomorphization_collector_graph_walk", || {
            par_iter(roots).for_each(|root| {
                let mut recursion_depths = DefIdMap::default();
                let hooks = GotocHooks::default();

                collect_items_rec(
                    tcx,
                    dummy_spanned(root),
                    visited,
                    &mut recursion_depths,
                    inlining_map,
                    &hooks,
                );
            });
        });
    }

    (visited.into_inner(), inlining_map.into_inner())
}

// Find all non-generic items by walking the HIR. These items serve as roots to
// start monomorphizing from.
fn collect_roots(tcx: TyCtxt<'_>, mode: MonoItemCollectionMode) -> Vec<MonoItem<'_>> {
    debug!("collecting roots");
    let mut roots = Vec::new();

    {
        let entry_fn = tcx.entry_fn(()).and_then(|(id, ty)| Some((id.expect_local(), ty)));

        debug!("collect_roots: entry_fn = {:?}", entry_fn);

        let hooks = GotocHooks::default();
        let mut visitor = RootCollector { tcx, mode, entry_fn, hooks, output: &mut roots };

        tcx.hir().krate().visit_all_item_likes(&mut visitor);

        // if we are generating an executable, then collector would originally insert starter code,
        // which is commented out in this version.
        // visitor.push_extra_entry_roots();
    }

    // We can only codegen items that are instantiable - items all of
    // whose predicates hold. Luckily, items that aren't instantiable
    // can't actually be used, so we can just skip codegenning them.
    roots
        .into_iter()
        .filter_map(|root| root.node.is_instantiable(tcx).then_some(root.node))
        .collect()
}

// Collect all monomorphized items reachable from `starting_point`
fn collect_items_rec<'tcx>(
    tcx: TyCtxt<'tcx>,
    starting_point: Spanned<MonoItem<'tcx>>,
    visited: MTRef<'_, MTLock<FxHashSet<MonoItem<'tcx>>>>,
    recursion_depths: &mut DefIdMap<usize>,
    inlining_map: MTRef<'_, MTLock<InliningMap<'tcx>>>,
    hooks: &GotocHooks<'tcx>,
) {
    if !visited.lock_mut().insert(starting_point.node) {
        // We've been here already, no need to search again.
        return;
    }
    debug!("BEGIN collect_items_rec({})", starting_point.node.to_string());

    let mut neighbors = Vec::new();
    let recursion_depth_reset;

    match starting_point.node {
        MonoItem::Static(def_id) => {
            let instance = Instance::mono(tcx, def_id);

            // Sanity check whether this ended up being collected accidentally
            debug_assert!(should_codegen_locally(tcx, &instance));

            let ty = instance.ty(tcx, ty::ParamEnv::reveal_all());
            visit_drop_use(tcx, ty, true, hooks, starting_point.span, &mut neighbors);

            recursion_depth_reset = None;

            if let Ok(alloc) = tcx.eval_static_initializer(def_id) {
                for &id in alloc.relocations().values() {
                    collect_miri(tcx, id, &mut neighbors);
                }
            }
        }
        MonoItem::Fn(instance) => {
            // Sanity check whether this ended up being collected accidentally
            debug_assert!(should_codegen_locally(tcx, &instance));

            // Keep track of the monomorphization recursion depth
            recursion_depth_reset =
                Some(check_recursion_limit(tcx, instance, starting_point.span, recursion_depths));
            check_type_length_limit(tcx, instance);

            rustc_data_structures::stack::ensure_sufficient_stack(|| {
                collect_neighbours(tcx, instance, hooks, &mut neighbors);
            });
        }
        MonoItem::GlobalAsm(..) => {
            recursion_depth_reset = None;
        }
    }

    record_accesses(tcx, starting_point.node, neighbors.iter().map(|i| &i.node), inlining_map);

    for neighbour in neighbors {
        collect_items_rec(tcx, neighbour, visited, recursion_depths, inlining_map, hooks);
    }

    if let Some((def_id, depth)) = recursion_depth_reset {
        recursion_depths.insert(def_id, depth);
    }

    debug!("END collect_items_rec({})", starting_point.node.to_string());
}

fn record_accesses<'a, 'tcx: 'a>(
    tcx: TyCtxt<'tcx>,
    caller: MonoItem<'tcx>,
    callees: impl Iterator<Item = &'a MonoItem<'tcx>>,
    inlining_map: MTRef<'_, MTLock<InliningMap<'tcx>>>,
) {
    let is_inlining_candidate = |mono_item: &MonoItem<'tcx>| {
        mono_item.instantiation_mode(tcx) == InstantiationMode::LocalCopy
    };

    // We collect this into a `SmallVec` to avoid calling `is_inlining_candidate` in the lock.
    // FIXME: Call `is_inlining_candidate` when pushing to `neighbors` in `collect_items_rec`
    // instead to avoid creating this `SmallVec`.
    let accesses: SmallVec<[_; 128]> =
        callees.map(|mono_item| (*mono_item, is_inlining_candidate(mono_item))).collect();

    inlining_map.lock_mut().record_accesses(caller, &accesses);
}

fn check_recursion_limit<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    span: Span,
    recursion_depths: &mut DefIdMap<usize>,
) -> (DefId, usize) {
    let def_id = instance.def_id();
    let recursion_depth = recursion_depths.get(&def_id).cloned().unwrap_or(0);
    debug!(" => recursion depth={}", recursion_depth);

    let adjusted_recursion_depth = if Some(def_id) == tcx.lang_items().drop_in_place_fn() {
        // HACK: drop_in_place creates tight monomorphization loops. Give
        // it more margin.
        recursion_depth / 4
    } else {
        recursion_depth
    };

    // Code that needs to instantiate the same function recursively
    // more than the recursion limit is assumed to be causing an
    // infinite expansion.
    if !tcx.recursion_limit().value_within_limit(adjusted_recursion_depth) {
        let error = format!("reached the recursion limit while instantiating `{}`", instance);
        let mut err = tcx.sess.struct_span_fatal(span, &error);
        err.span_note(
            tcx.def_span(def_id),
            &format!("`{}` defined here", with_no_trimmed_paths(|| tcx.def_path_str(def_id))),
        );
        err.emit();
        FatalError.raise();
    }

    recursion_depths.insert(def_id, recursion_depth + 1);

    (def_id, recursion_depth)
}

fn check_type_length_limit<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) {
    let type_length = instance
        .substs
        .iter()
        .flat_map(|arg| arg.walk(tcx))
        .filter(|arg| match arg.unpack() {
            GenericArgKind::Type(_) | GenericArgKind::Const(_) => true,
            GenericArgKind::Lifetime(_) => false,
        })
        .count();
    debug!(" => type length={}", type_length);

    // Rust code can easily create exponentially-long types using only a
    // polynomial recursion depth. Even with the default recursion
    // depth, you can easily get cases that take >2^60 steps to run,
    // which means that rustc basically hangs.
    //
    // Bail out in these cases to avoid that bad user experience.
    if !tcx.type_length_limit().value_within_limit(type_length) {
        // The instance name is already known to be too long for rustc.
        // Show only the first and last 32 characters to avoid blasting
        // the user's terminal with thousands of lines of type-name.
        let shrink = |s: String, before: usize, after: usize| {
            // An iterator of all byte positions including the end of the string.
            let positions = || s.char_indices().map(|(i, _)| i).chain(iter::once(s.len()));

            let shrunk = format!(
                "{before}...{after}",
                before = &s[..positions().nth(before).unwrap_or(s.len())],
                after = &s[positions().rev().nth(after).unwrap_or(0)..],
            );

            // Only use the shrunk version if it's really shorter.
            // This also avoids the case where before and after slices overlap.
            if shrunk.len() < s.len() { shrunk } else { s }
        };
        let msg = format!(
            "reached the type-length limit while instantiating `{}`",
            shrink(instance.to_string(), 32, 32)
        );
        let mut diag = tcx.sess.struct_span_fatal(tcx.def_span(instance.def_id()), &msg);
        diag.note(&format!(
            "consider adding a `#![type_length_limit=\"{}\"]` attribute to your crate",
            type_length
        ));
        diag.emit();
        tcx.sess.abort_if_errors();
    }
}

struct MirNeighborCollector<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    body: &'a mir::Body<'tcx>,
    output: &'a mut Vec<Spanned<MonoItem<'tcx>>>,
    instance: Instance<'tcx>,
    hooks: &'a GotocHooks<'tcx>,
}

impl<'a, 'tcx> MirNeighborCollector<'a, 'tcx> {
    pub fn monomorphize<T>(&self, value: T) -> T
    where
        T: TypeFoldable<'tcx>,
    {
        debug!("monomorphize: self.instance={:?}", self.instance);
        self.instance.subst_mir_and_normalize_erasing_regions(
            self.tcx,
            ty::ParamEnv::reveal_all(),
            value,
        )
    }
}

impl<'a, 'tcx> MirVisitor<'tcx> for MirNeighborCollector<'a, 'tcx> {
    fn visit_rvalue(&mut self, rvalue: &mir::Rvalue<'tcx>, location: Location) {
        debug!("visiting rvalue {:?}", *rvalue);

        let span = self.body.source_info(location).span;

        match *rvalue {
            // When doing an cast from a regular pointer to a fat pointer, we
            // have to instantiate all methods of the trait being cast to, so we
            // can build the appropriate vtable.
            mir::Rvalue::Cast(
                mir::CastKind::Pointer(PointerCast::Unsize),
                ref operand,
                target_ty,
            ) => {
                let target_ty = self.monomorphize(target_ty);
                let source_ty = operand.ty(self.body, self.tcx);
                let source_ty = self.monomorphize(source_ty);
                let (source_ty, target_ty) =
                    find_vtable_types_for_unsizing(self.tcx, source_ty, target_ty);
                // This could also be a different Unsize instruction, like
                // from a fixed sized array to a slice. But we are only
                // interested in things that produce a vtable.
                if target_ty.is_trait() && !source_ty.is_trait() {
                    create_mono_items_for_vtable_methods(
                        self.tcx,
                        target_ty,
                        source_ty,
                        &self.hooks,
                        span,
                        self.output,
                    );
                }
            }
            mir::Rvalue::Cast(
                mir::CastKind::Pointer(PointerCast::ReifyFnPointer),
                ref operand,
                _,
            ) => {
                let fn_ty = operand.ty(self.body, self.tcx);
                let fn_ty = self.monomorphize(fn_ty);
                visit_fn_use(self.tcx, fn_ty, false, &self.hooks, span, &mut self.output);
            }
            mir::Rvalue::Cast(
                mir::CastKind::Pointer(PointerCast::ClosureFnPointer(_)),
                ref operand,
                _,
            ) => {
                let source_ty = operand.ty(self.body, self.tcx);
                let source_ty = self.monomorphize(source_ty);
                match source_ty.kind() {
                    ty::Closure(def_id, substs) => {
                        let instance = Instance::resolve_closure(
                            self.tcx,
                            *def_id,
                            substs,
                            ty::ClosureKind::FnOnce,
                        );
                        if should_codegen_locally(self.tcx, &instance) {
                            self.output.push(create_fn_mono_item(instance, span));
                        }
                    }
                    _ => unreachable!(),
                }
            }
            mir::Rvalue::NullaryOp(mir::NullOp::Box, _) => {
                let tcx = self.tcx;
                let exchange_malloc_fn_def_id =
                    tcx.require_lang_item(LangItem::ExchangeMalloc, None);
                let instance = Instance::mono(tcx, exchange_malloc_fn_def_id);
                if should_codegen_locally(tcx, &instance) {
                    self.output.push(create_fn_mono_item(instance, span));
                }
            }
            mir::Rvalue::ThreadLocalRef(def_id) => {
                assert!(self.tcx.is_thread_local_static(def_id));
                let instance = Instance::mono(self.tcx, def_id);
                if should_codegen_locally(self.tcx, &instance) {
                    trace!("collecting thread-local static {:?}", def_id);
                    self.output.push(respan(span, MonoItem::Static(def_id)));
                }
            }
            _ => { /* not interesting */ }
        }

        self.super_rvalue(rvalue, location);
    }

    fn visit_const(&mut self, constant: &&'tcx ty::Const<'tcx>, location: Location) {
        debug!("visiting const {:?} @ {:?}", *constant, location);

        let substituted_constant = self.monomorphize(*constant);
        let param_env = ty::ParamEnv::reveal_all();

        match substituted_constant.val {
            ty::ConstKind::Value(val) => collect_const_value(self.tcx, val, self.output),
            ty::ConstKind::Unevaluated(unevaluated) => {
                match self.tcx.const_eval_resolve(param_env, unevaluated, None) {
                    Ok(val) => collect_const_value(self.tcx, val, self.output),
                    Err(ErrorHandled::Reported(ErrorReported) | ErrorHandled::Linted) => {}
                    Err(ErrorHandled::TooGeneric) => unreachable!(
                        "collection encountered polymorphic constant: {}",
                        substituted_constant
                    ),
                }
            }
            _ => {}
        }

        self.super_const(constant);
    }

    fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, location: Location) {
        debug!("visiting terminator {:?} @ {:?}", terminator, location);
        let source = self.body.source_info(location).span;

        let tcx = self.tcx;
        match terminator.kind {
            mir::TerminatorKind::Call { ref func, .. } => {
                let callee_ty = func.ty(self.body, tcx);
                let callee_ty = self.monomorphize(callee_ty);
                visit_fn_use(self.tcx, callee_ty, true, &self.hooks, source, &mut self.output);
            }
            mir::TerminatorKind::Drop { ref place, .. }
            | mir::TerminatorKind::DropAndReplace { ref place, .. } => {
                let ty = place.ty(self.body, self.tcx).ty;
                let ty = self.monomorphize(ty);
                visit_drop_use(self.tcx, ty, true, &self.hooks, source, self.output);
            }
            mir::TerminatorKind::InlineAsm { ref operands, .. } => {
                for op in operands {
                    match *op {
                        mir::InlineAsmOperand::SymFn { ref value } => {
                            let fn_ty = self.monomorphize(value.literal.ty());
                            visit_fn_use(
                                self.tcx,
                                fn_ty,
                                false,
                                &self.hooks,
                                source,
                                &mut self.output,
                            );
                        }
                        mir::InlineAsmOperand::SymStatic { def_id } => {
                            let instance = Instance::mono(self.tcx, def_id);
                            if should_codegen_locally(self.tcx, &instance) {
                                trace!("collecting asm sym static {:?}", def_id);
                                self.output.push(respan(source, MonoItem::Static(def_id)));
                            }
                        }
                        _ => {}
                    }
                }
            }
            mir::TerminatorKind::Goto { .. }
            | mir::TerminatorKind::SwitchInt { .. }
            | mir::TerminatorKind::Resume
            | mir::TerminatorKind::Abort
            | mir::TerminatorKind::Return
            | mir::TerminatorKind::Unreachable
            | mir::TerminatorKind::Assert { .. } => {}
            mir::TerminatorKind::GeneratorDrop
            | mir::TerminatorKind::Yield { .. }
            | mir::TerminatorKind::FalseEdge { .. }
            | mir::TerminatorKind::FalseUnwind { .. } => unreachable!(),
        }

        self.super_terminator(terminator, location);
    }

    fn visit_local(
        &mut self,
        _place_local: &Local,
        _context: mir::visit::PlaceContext,
        _location: Location,
    ) {
    }
}

fn visit_drop_use<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    is_direct_call: bool,
    hooks: &GotocHooks<'tcx>,
    source: Span,
    output: &mut Vec<Spanned<MonoItem<'tcx>>>,
) {
    let instance = Instance::resolve_drop_in_place(tcx, ty);
    visit_instance_use(tcx, instance, is_direct_call, hooks, source, output);
}

fn visit_fn_use<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    is_direct_call: bool,
    hooks: &GotocHooks<'tcx>,
    source: Span,
    output: &mut Vec<Spanned<MonoItem<'tcx>>>,
) {
    if let ty::FnDef(def_id, substs) = ty.kind() {
        let instance = if is_direct_call {
            ty::Instance::resolve(tcx, ty::ParamEnv::reveal_all(), *def_id, substs)
                .unwrap()
                .unwrap()
        } else {
            ty::Instance::resolve_for_fn_ptr(tcx, ty::ParamEnv::reveal_all(), *def_id, substs)
                .unwrap()
        };
        visit_instance_use(tcx, instance, is_direct_call, hooks, source, output);
    }
}

fn visit_instance_use<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: ty::Instance<'tcx>,
    is_direct_call: bool,
    hooks: &GotocHooks<'tcx>,
    source: Span,
    output: &mut Vec<Spanned<MonoItem<'tcx>>>,
) {
    debug!("visit_item_use({:?}, is_direct_call={:?})", instance, is_direct_call);
    if !should_codegen_locally(tcx, &instance) {
        return;
    }

    // we ignore functions to which hooks may apply
    if hooks.hook_applies(tcx, instance).is_some() {
        return;
    }

    match instance.def {
        ty::InstanceDef::Virtual(..) | ty::InstanceDef::Intrinsic(_) => {
            if !is_direct_call {
                unreachable!("{:?} being reified", instance);
            }
        }
        ty::InstanceDef::DropGlue(_, None) => {
            // Don't need to emit noop drop glue if we are calling directly.
            if !is_direct_call {
                output.push(create_fn_mono_item(instance, source));
            }
        }
        ty::InstanceDef::DropGlue(_, Some(_))
        | ty::InstanceDef::VtableShim(..)
        | ty::InstanceDef::ReifyShim(..)
        | ty::InstanceDef::ClosureOnceShim { .. }
        | ty::InstanceDef::Item(..)
        | ty::InstanceDef::FnPtrShim(..)
        | ty::InstanceDef::CloneShim(..) => {
            output.push(create_fn_mono_item(instance, source));
        }
    }
}

// Returns `true` if we should codegen an instance in the local crate.
// Returns `false` if we can just link to the upstream crate and therefore don't
// need a mono item.
fn should_codegen_locally<'tcx>(tcx: TyCtxt<'tcx>, instance: &Instance<'tcx>) -> bool {
    let def_id = match instance.def {
        ty::InstanceDef::Item(def) => def.did,
        ty::InstanceDef::DropGlue(def_id, Some(_)) => def_id,
        ty::InstanceDef::VtableShim(..)
        | ty::InstanceDef::ReifyShim(..)
        | ty::InstanceDef::ClosureOnceShim { .. }
        | ty::InstanceDef::Virtual(..)
        | ty::InstanceDef::FnPtrShim(..)
        | ty::InstanceDef::DropGlue(..)
        | ty::InstanceDef::Intrinsic(_)
        | ty::InstanceDef::CloneShim(..) => return true,
    };

    if tcx.is_foreign_item(def_id) {
        // Foreign items are always linked against, there's no way of
        // instantiating them.
        return false;
    }

    if def_id.is_local() {
        // Local items cannot be referred to locally without
        // monomorphizing them locally.
        return true;
    }

    if tcx.is_reachable_non_generic(def_id) || instance.upstream_monomorphization(tcx).is_some() {
        // We can link to the item in question, no instance needed
        // in this crate.
        return false;
    }

    if !tcx.is_mir_available(def_id) {
        unreachable!("cannot create local mono-item for {:?}", def_id)
    }

    true
}

/// For a given pair of source and target type that occur in an unsizing coercion,
/// this function finds the pair of types that determines the vtable linking
/// them.
///
/// For example, the source type might be `&SomeStruct` and the target type\
/// might be `&SomeTrait` in a cast like:
///
/// let src: &SomeStruct = ...;
/// let target = src as &SomeTrait;
///
/// Then the output of this function would be (SomeStruct, SomeTrait) since for
/// constructing the `target` fat-pointer we need the vtable for that pair.
///
/// Things can get more complicated though because there's also the case where
/// the unsized type occurs as a field:
///
/// ```rust
/// struct ComplexStruct<T: ?Sized> {
///    a: u32,
///    b: f64,
///    c: T
/// }
/// ```
///
/// In this case, if `T` is sized, `&ComplexStruct<T>` is a thin pointer. If `T`
/// is unsized, `&SomeStruct` is a fat pointer, and the vtable it points to is
/// for the pair of `T` (which is a trait) and the concrete type that `T` was
/// originally coerced from:
///
/// let src: &ComplexStruct<SomeStruct> = ...;
/// let target = src as &ComplexStruct<SomeTrait>;
///
/// Again, we want this `find_vtable_types_for_unsizing()` to provide the pair
/// `(SomeStruct, SomeTrait)`.
///
/// Finally, there is also the case of custom unsizing coercions, e.g., for
/// smart pointers such as `Rc` and `Arc`.
fn find_vtable_types_for_unsizing<'tcx>(
    tcx: TyCtxt<'tcx>,
    source_ty: Ty<'tcx>,
    target_ty: Ty<'tcx>,
) -> (Ty<'tcx>, Ty<'tcx>) {
    let ptr_vtable = |inner_source: Ty<'tcx>, inner_target: Ty<'tcx>| {
        let param_env = ty::ParamEnv::reveal_all();
        let type_has_metadata = |ty: Ty<'tcx>| -> bool {
            if ty.is_sized(tcx.at(DUMMY_SP), param_env) {
                return false;
            }
            let tail = tcx.struct_tail_erasing_lifetimes(ty, param_env);
            match tail.kind() {
                ty::Foreign(..) => false,
                ty::Str | ty::Slice(..) | ty::Dynamic(..) => true,
                _ => unreachable!("unexpected unsized tail: {:?}", tail),
            }
        };
        if type_has_metadata(inner_source) {
            (inner_source, inner_target)
        } else {
            tcx.struct_lockstep_tails_erasing_lifetimes(inner_source, inner_target, param_env)
        }
    };

    match (&source_ty.kind(), &target_ty.kind()) {
        (&ty::Ref(_, a, _), &ty::Ref(_, b, _) | &ty::RawPtr(ty::TypeAndMut { ty: b, .. }))
        | (&ty::RawPtr(ty::TypeAndMut { ty: a, .. }), &ty::RawPtr(ty::TypeAndMut { ty: b, .. })) => {
            ptr_vtable(a, b)
        }
        (&ty::Adt(def_a, _), &ty::Adt(def_b, _)) if def_a.is_box() && def_b.is_box() => {
            ptr_vtable(source_ty.boxed_ty(), target_ty.boxed_ty())
        }

        (&ty::Adt(source_adt_def, source_substs), &ty::Adt(target_adt_def, target_substs)) => {
            assert_eq!(source_adt_def, target_adt_def);

            let CustomCoerceUnsized::Struct(coerce_index) =
                custom_coerce_unsize_info(tcx, source_ty, target_ty);

            let source_fields = &source_adt_def.non_enum_variant().fields;
            let target_fields = &target_adt_def.non_enum_variant().fields;

            assert!(
                coerce_index < source_fields.len() && source_fields.len() == target_fields.len()
            );

            find_vtable_types_for_unsizing(
                tcx,
                source_fields[coerce_index].ty(tcx, source_substs),
                target_fields[coerce_index].ty(tcx, target_substs),
            )
        }
        _ => unreachable!(
            "find_vtable_types_for_unsizing: invalid coercion {:?} -> {:?}",
            source_ty, target_ty
        ),
    }
}

fn create_fn_mono_item(instance: Instance<'_>, source: Span) -> Spanned<MonoItem<'_>> {
    debug!("create_fn_mono_item(instance={})", instance);
    respan(source, MonoItem::Fn(instance))
}

/// Creates a `MonoItem` for each method that is referenced by the vtable for
/// the given trait/impl pair.
fn create_mono_items_for_vtable_methods<'tcx>(
    tcx: TyCtxt<'tcx>,
    trait_ty: Ty<'tcx>,
    impl_ty: Ty<'tcx>,
    hooks: &GotocHooks<'tcx>,
    source: Span,
    output: &mut Vec<Spanned<MonoItem<'tcx>>>,
) {
    assert!(!trait_ty.has_escaping_bound_vars() && !impl_ty.has_escaping_bound_vars());

    if let ty::Dynamic(ref trait_ty, ..) = trait_ty.kind() {
        if let Some(principal) = trait_ty.principal() {
            let poly_trait_ref = principal.with_self_ty(tcx, impl_ty);
            assert!(!poly_trait_ref.has_escaping_bound_vars());

            // Walk all methods of the trait, including those of its supertraits
            let methods = tcx.vtable_entries(poly_trait_ref);
            let methods = methods
                .iter()
                .cloned()
                .filter_map(|entry| match entry {
                    VtblEntry::Method(instance) => Some(instance),
                    VtblEntry::MetadataDropInPlace
                    | VtblEntry::MetadataSize
                    | VtblEntry::MetadataAlign
                    | VtblEntry::Vacant => None,
                    // Super trait vtable entries already handled by now
                    VtblEntry::TraitVPtr(..) => None,
                })
                .filter(|&instance| should_codegen_locally(tcx, &instance))
                .map(|item| create_fn_mono_item(item, source));
            output.extend(methods);
        }

        // Also add the destructor.
        visit_drop_use(tcx, impl_ty, false, hooks, source, output);
    }
}

//=-----------------------------------------------------------------------------
// Root Collection
//=-----------------------------------------------------------------------------

struct RootCollector<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    mode: MonoItemCollectionMode,
    output: &'a mut Vec<Spanned<MonoItem<'tcx>>>,
    entry_fn: Option<(LocalDefId, EntryFnType)>,
    hooks: GotocHooks<'tcx>,
}

impl ItemLikeVisitor<'v> for RootCollector<'_, 'v> {
    // This is treated as a no-op here:
    // compiler/rustc_mir/src/monomorphize/collector.rs
    fn visit_foreign_item(&mut self, _: &'hir rustc_hir::ForeignItem<'hir>) {}

    fn visit_item(&mut self, item: &'v hir::Item<'v>) {
        match item.kind {
            hir::ItemKind::ExternCrate(..)
            | hir::ItemKind::Use(..)
            | hir::ItemKind::Macro(..)
            | hir::ItemKind::ForeignMod { .. }
            | hir::ItemKind::TyAlias(..)
            | hir::ItemKind::Trait(..)
            | hir::ItemKind::TraitAlias(..)
            | hir::ItemKind::OpaqueTy(..)
            | hir::ItemKind::Mod(..) => {
                // Nothing to do, just keep recursing.
            }

            hir::ItemKind::Impl { .. } => {
                if self.mode == MonoItemCollectionMode::Eager {
                    create_mono_items_for_default_impls(self.tcx, item, self.output);
                }
            }

            hir::ItemKind::Enum(_, ref generics)
            | hir::ItemKind::Struct(_, ref generics)
            | hir::ItemKind::Union(_, ref generics) => {
                if generics.params.is_empty() {
                    if self.mode == MonoItemCollectionMode::Eager {
                        debug!(
                            "RootCollector: ADT drop-glue for {}",
                            self.tcx.def_path_str(item.def_id.to_def_id())
                        );

                        let ty = Instance::new(item.def_id.to_def_id(), InternalSubsts::empty())
                            .ty(self.tcx, ty::ParamEnv::reveal_all());
                        visit_drop_use(self.tcx, ty, true, &self.hooks, DUMMY_SP, self.output);
                    }
                }
            }
            hir::ItemKind::GlobalAsm(..) => {
                debug!(
                    "RootCollector: ItemKind::GlobalAsm({})",
                    self.tcx.def_path_str(item.def_id.to_def_id())
                );
                self.output.push(dummy_spanned(MonoItem::GlobalAsm(item.item_id())));
            }
            hir::ItemKind::Static(..) => {
                debug!(
                    "RootCollector: ItemKind::Static({})",
                    self.tcx.def_path_str(item.def_id.to_def_id())
                );
                self.output.push(dummy_spanned(MonoItem::Static(item.def_id.to_def_id())));
            }
            hir::ItemKind::Const(..) => {
                // const items only generate mono items if they are
                // actually used somewhere. Just declaring them is insufficient.

                // but even just declaring them must collect the items they refer to
                if let Ok(val) = self.tcx.const_eval_poly(item.def_id.to_def_id()) {
                    collect_const_value(self.tcx, val, &mut self.output);
                }
            }
            hir::ItemKind::Fn(..) => {
                self.push_if_root(item.def_id);
            }
        }
    }

    fn visit_trait_item(&mut self, _: &'v hir::TraitItem<'v>) {
        // Even if there's a default body with no explicit generics,
        // it's still generic over some `Self: Trait`, so not a root.
    }

    fn visit_impl_item(&mut self, ii: &'v hir::ImplItem<'v>) {
        if let hir::ImplItemKind::Fn(hir::FnSig { .. }, _) = ii.kind {
            self.push_if_root(ii.def_id);
        }
    }
}

impl RootCollector<'_, 'v> {
    fn is_root(&self, def_id: LocalDefId) -> bool {
        !item_requires_monomorphization(self.tcx, def_id)
            && match self.mode {
                MonoItemCollectionMode::Eager => true,
                MonoItemCollectionMode::Lazy => {
                    self.entry_fn.map(|(id, _)| id) == Some(def_id)
                        || self.tcx.is_reachable_non_generic(def_id)
                        || self
                            .tcx
                            .codegen_fn_attrs(def_id)
                            .flags
                            .contains(CodegenFnAttrFlags::RUSTC_STD_INTERNAL_SYMBOL)
                }
            }
    }

    /// If `def_id` represents a root, pushes it onto the list of
    /// outputs. (Note that all roots must be monomorphic.)
    fn push_if_root(&mut self, def_id: LocalDefId) {
        if self.is_root(def_id) {
            debug!("RootCollector::push_if_root: found root def_id={:?}", def_id);

            let instance = Instance::mono(self.tcx, def_id.to_def_id());
            self.output.push(create_fn_mono_item(instance, DUMMY_SP));
        }
    }
}

fn item_requires_monomorphization(tcx: TyCtxt<'_>, def_id: LocalDefId) -> bool {
    let generics = tcx.generics_of(def_id);
    generics.requires_monomorphization(tcx)
}

fn create_mono_items_for_default_impls<'tcx>(
    tcx: TyCtxt<'tcx>,
    item: &'tcx hir::Item<'tcx>,
    output: &mut Vec<Spanned<MonoItem<'tcx>>>,
) {
    match item.kind {
        hir::ItemKind::Impl(ref impl_) => {
            for param in impl_.generics.params {
                match param.kind {
                    hir::GenericParamKind::Lifetime { .. } => {}
                    hir::GenericParamKind::Type { .. } | hir::GenericParamKind::Const { .. } => {
                        return;
                    }
                }
            }

            debug!(
                "create_mono_items_for_default_impls(item={})",
                tcx.def_path_str(item.def_id.to_def_id())
            );

            if let Some(trait_ref) = tcx.impl_trait_ref(item.def_id) {
                let param_env = ty::ParamEnv::reveal_all();
                let trait_ref = tcx.normalize_erasing_regions(param_env, trait_ref);
                let overridden_methods: FxHashSet<_> =
                    impl_.items.iter().map(|iiref| iiref.ident.normalize_to_macros_2_0()).collect();
                for method in tcx.provided_trait_methods(trait_ref.def_id) {
                    if overridden_methods.contains(&method.ident.normalize_to_macros_2_0()) {
                        continue;
                    }

                    if tcx.generics_of(method.def_id).own_requires_monomorphization() {
                        continue;
                    }

                    let substs =
                        InternalSubsts::for_item(tcx, method.def_id, |param, _| match param.kind {
                            GenericParamDefKind::Lifetime => tcx.lifetimes.re_erased.into(),
                            GenericParamDefKind::Type { .. }
                            | GenericParamDefKind::Const { .. } => {
                                trait_ref.substs[param.index as usize]
                            }
                        });
                    let instance = ty::Instance::resolve(tcx, param_env, method.def_id, substs)
                        .unwrap()
                        .unwrap();

                    let mono_item = create_fn_mono_item(instance, DUMMY_SP);
                    if mono_item.node.is_instantiable(tcx) && should_codegen_locally(tcx, &instance)
                    {
                        output.push(mono_item);
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}

/// Scans the miri alloc in order to find function calls, closures, and drop-glue.
fn collect_miri<'tcx>(
    tcx: TyCtxt<'tcx>,
    alloc_id: AllocId,
    output: &mut Vec<Spanned<MonoItem<'tcx>>>,
) {
    match tcx.global_alloc(alloc_id) {
        GlobalAlloc::Static(def_id) => {
            assert!(!tcx.is_thread_local_static(def_id));
            let instance = Instance::mono(tcx, def_id);
            if should_codegen_locally(tcx, &instance) {
                trace!("collecting static {:?}", def_id);
                output.push(dummy_spanned(MonoItem::Static(def_id)));
            }
        }
        GlobalAlloc::Memory(alloc) => {
            trace!("collecting {:?} with {:#?}", alloc_id, alloc);
            for &inner in alloc.relocations().values() {
                rustc_data_structures::stack::ensure_sufficient_stack(|| {
                    collect_miri(tcx, inner, output);
                });
            }
        }
        GlobalAlloc::Function(fn_instance) => {
            if should_codegen_locally(tcx, &fn_instance) {
                trace!("collecting {:?} with {:#?}", alloc_id, fn_instance);
                output.push(create_fn_mono_item(fn_instance, DUMMY_SP));
            }
        }
    }
}

/// Scans the MIR in order to find function calls, closures, and drop-glue.
fn collect_neighbours<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    hooks: &GotocHooks<'tcx>,
    output: &mut Vec<Spanned<MonoItem<'tcx>>>,
) {
    debug!("collect_neighbours: {:?}", instance.def_id());
    let body = tcx.instance_mir(instance.def);

    MirNeighborCollector { tcx, body: &body, output, instance, hooks }.visit_body(&body);
}

fn collect_const_value<'tcx>(
    tcx: TyCtxt<'tcx>,
    value: ConstValue<'tcx>,
    output: &mut Vec<Spanned<MonoItem<'tcx>>>,
) {
    match value {
        ConstValue::Scalar(Scalar::Ptr(ptr, _size)) => {
            let (alloc_id, _offset) = ptr.into_parts();
            collect_miri(tcx, alloc_id, output);
        }
        ConstValue::Slice { data: alloc, start: _, end: _ } | ConstValue::ByRef { alloc, .. } => {
            for &id in alloc.relocations().values() {
                collect_miri(tcx, id, output);
            }
        }
        _ => {}
    }
}
