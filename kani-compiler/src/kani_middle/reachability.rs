// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module implements a cross-crate collector that allow us to find all items that
//! should be included in order to verify one or more proof harness.
//!
//! This module works as following:
//!   - Traverse all reachable items starting at the given starting points.
//!   - For every function, traverse its body and collect the following:
//!     - Constants / Static objects.
//!     - Functions that are called or have their address taken.
//!     - VTable methods for types that are coerced as unsized types.
//!   - For every static, collect initializer and drop functions.
//!
//! We have kept this module agnostic of any Kani code in case we can contribute this back to rustc.
use tracing::{debug, debug_span, trace, warn};

use rustc_data_structures::fingerprint::Fingerprint;
use rustc_data_structures::fx::FxHashSet;
use rustc_data_structures::stable_hasher::{HashStable, StableHasher};
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_hir::ItemId;
use rustc_middle::mir::interpret::{AllocId, ConstValue, ErrorHandled, GlobalAlloc, Scalar};
use rustc_middle::mir::mono::MonoItem;
use rustc_middle::mir::visit::Visitor as MirVisitor;
use rustc_middle::mir::{
    Body, CastKind, Constant, ConstantKind, Location, Rvalue, Terminator, TerminatorKind,
};
use rustc_middle::span_bug;
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{
    Closure, ClosureKind, ConstKind, EarlyBinder, Instance, InstanceDef, ParamEnv, Ty, TyCtxt,
    TyKind, TypeFoldable, VtblEntry,
};

use crate::kani_middle::coercion;
use crate::kani_middle::stubbing::get_stub;

/// Collect all reachable items starting from the given starting points.
pub fn collect_reachable_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    starting_points: &[MonoItem<'tcx>],
) -> Vec<MonoItem<'tcx>> {
    // For each harness, collect items using the same collector.
    // I.e.: This will return any item that is reachable from one or more of the starting points.
    let mut collector = MonoItemsCollector::new(tcx);
    for item in starting_points {
        collector.collect(*item);
    }

    #[cfg(debug_assertions)]
    collector
        .call_graph
        .dump_dot(tcx)
        .unwrap_or_else(|e| tracing::error!("Failed to dump call graph: {e}"));

    tcx.sess.abort_if_errors();

    // Sort the result so code generation follows deterministic order.
    // This helps us to debug the code, but it also provides the user a good experience since the
    // order of the errors and warnings is stable.
    let mut sorted_items: Vec<_> = collector.collected.into_iter().collect();
    sorted_items.sort_by_cached_key(|item| to_fingerprint(tcx, item));
    sorted_items
}

/// Collect all (top-level) items in the crate that matches the given predicate.
/// An item can only be a root if they are: non-generic Fn / Static / GlobalASM
pub fn filter_crate_items<F>(tcx: TyCtxt, predicate: F) -> Vec<MonoItem>
where
    F: Fn(TyCtxt, DefId) -> bool,
{
    let crate_items = tcx.hir_crate_items(());
    // Filter regular items.
    let root_items = crate_items.items().filter_map(|item| {
        let def_id = item.owner_id.def_id.to_def_id();
        if !is_generic(tcx, def_id) && predicate(tcx, def_id) {
            to_mono_root(tcx, item, def_id)
        } else {
            None
        }
    });

    // Filter items from implementation blocks.
    let impl_items = crate_items.impl_items().filter_map(|impl_item| {
        let def_id = impl_item.owner_id.def_id.to_def_id();
        if matches!(tcx.def_kind(def_id), DefKind::AssocFn)
            && !is_generic(tcx, def_id)
            && predicate(tcx, def_id)
        {
            Some(MonoItem::Fn(Instance::mono(tcx, def_id)))
        } else {
            None
        }
    });
    root_items.chain(impl_items).collect()
}

/// Use a predicate to find `const` declarations, then extract all items reachable from them.
///
/// Probably only specifically useful with a predicate to find `TestDescAndFn` const declarations from
/// tests and extract the closures from them.
pub fn filter_const_crate_items<F>(tcx: TyCtxt, mut predicate: F) -> Vec<MonoItem>
where
    F: FnMut(TyCtxt, DefId) -> bool,
{
    let mut roots = Vec::new();
    for hir_id in tcx.hir_crate_items(()).items() {
        let def_id = hir_id.owner_id.def_id.to_def_id();
        let def_kind = tcx.def_kind(def_id);
        if matches!(def_kind, DefKind::Const) && predicate(tcx, def_id) {
            let instance = Instance::mono(tcx, def_id);
            let body = tcx.instance_mir(InstanceDef::Item(def_id));
            let mut collector =
                MonoItemsFnCollector { tcx, body, instance, collected: FxHashSet::default() };
            collector.visit_body(body);

            roots.extend(collector.collected);
        }
    }
    roots
}

fn is_generic(tcx: TyCtxt, def_id: DefId) -> bool {
    let generics = tcx.generics_of(def_id);
    generics.requires_monomorphization(tcx)
}

fn to_mono_root(tcx: TyCtxt, item_id: ItemId, def_id: DefId) -> Option<MonoItem> {
    let kind = tcx.def_kind(def_id);
    match kind {
        DefKind::Static(..) => Some(MonoItem::Static(def_id)),
        DefKind::Fn => Some(MonoItem::Fn(Instance::mono(tcx, def_id))),
        DefKind::GlobalAsm => Some(MonoItem::GlobalAsm(item_id)),
        _ => {
            debug!(?def_id, ?kind, "Ignored item. Not a root type.");
            None
        }
    }
}

struct MonoItemsCollector<'tcx> {
    /// The compiler context.
    tcx: TyCtxt<'tcx>,
    /// Set of collected items used to avoid entering recursion loops.
    collected: FxHashSet<MonoItem<'tcx>>,
    /// Items enqueued for visiting.
    queue: Vec<MonoItem<'tcx>>,
    #[cfg(debug_assertions)]
    call_graph: debug::CallGraph<'tcx>,
}

impl<'tcx> MonoItemsCollector<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        MonoItemsCollector {
            tcx,
            collected: FxHashSet::default(),
            queue: vec![],
            #[cfg(debug_assertions)]
            call_graph: debug::CallGraph::default(),
        }
    }
    /// Collects all reachable items starting from the given root.
    pub fn collect(&mut self, root: MonoItem<'tcx>) {
        debug!(?root, "collect");
        self.queue.push(root);
        self.reachable_items();
    }

    /// Traverses the call graph starting from the given root. For every function, we visit all
    /// instruction looking for the items that should be included in the compilation.
    fn reachable_items(&mut self) {
        while let Some(to_visit) = self.queue.pop() {
            if !self.collected.contains(&to_visit) {
                self.collected.insert(to_visit);
                let next_items = match to_visit {
                    MonoItem::Fn(instance) => self.visit_fn(instance),
                    MonoItem::Static(def_id) => self.visit_static(def_id),
                    MonoItem::GlobalAsm(_) => {
                        self.visit_asm(to_visit);
                        vec![]
                    }
                };
                #[cfg(debug_assertions)]
                self.call_graph.add_edges(to_visit, &next_items);

                self.queue
                    .extend(next_items.into_iter().filter(|item| !self.collected.contains(item)));
            }
        }
    }

    /// Visit a function and collect all mono-items reachable from its instructions.
    fn visit_fn(&mut self, instance: Instance<'tcx>) -> Vec<MonoItem<'tcx>> {
        let _guard = debug_span!("visit_fn", function=?instance).entered();
        let body = self.tcx.instance_mir(instance.def);
        let mut collector =
            MonoItemsFnCollector { tcx: self.tcx, collected: FxHashSet::default(), instance, body };
        collector.visit_body(body);
        collector.collected.into_iter().collect()
    }

    /// Visit a static object and collect drop / initialization functions.
    fn visit_static(&mut self, def_id: DefId) -> Vec<MonoItem<'tcx>> {
        let _guard = debug_span!("visit_static", ?def_id).entered();
        let instance = Instance::mono(self.tcx, def_id);
        let mut next_items = vec![];

        // Collect drop function.
        let static_ty = instance.ty(self.tcx, ParamEnv::reveal_all());
        let instance = Instance::resolve_drop_in_place(self.tcx, static_ty);
        next_items.push(MonoItem::Fn(instance.polymorphize(self.tcx)));

        // Collect initialization.
        let alloc = self.tcx.eval_static_initializer(def_id).unwrap();
        for id in alloc.inner().provenance().provenances() {
            next_items.extend(collect_alloc_items(self.tcx, id).iter());
        }

        next_items
    }

    /// Visit global assembly and collect its item.
    fn visit_asm(&mut self, item: MonoItem<'tcx>) {
        debug!(?item, "visit_asm");
    }
}

struct MonoItemsFnCollector<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    collected: FxHashSet<MonoItem<'tcx>>,
    instance: Instance<'tcx>,
    body: &'a Body<'tcx>,
}

impl<'a, 'tcx> MonoItemsFnCollector<'a, 'tcx> {
    fn monomorphize<T>(&self, value: T) -> T
    where
        T: TypeFoldable<TyCtxt<'tcx>>,
    {
        trace!(instance=?self.instance, ?value, "monomorphize");
        self.instance.subst_mir_and_normalize_erasing_regions(
            self.tcx,
            ParamEnv::reveal_all(),
            EarlyBinder::bind(value),
        )
    }

    /// Collect the implementation of all trait methods and its supertrait methods for the given
    /// concrete type.
    fn collect_vtable_methods(&mut self, concrete_ty: Ty<'tcx>, trait_ty: Ty<'tcx>) {
        trace!(?concrete_ty, ?trait_ty, "collect_vtable_methods");
        assert!(!concrete_ty.is_trait(), "Expected a concrete type, but found: {concrete_ty:?}");
        assert!(trait_ty.is_trait(), "Expected a trait: {trait_ty:?}");
        if let TyKind::Dynamic(trait_list, ..) = trait_ty.kind() {
            // A trait object type can have multiple trait bounds but up to one non-auto-trait
            // bound. This non-auto-trait, named principal, is the only one that can have methods.
            // https://doc.rust-lang.org/reference/special-types-and-traits.html#auto-traits
            if let Some(principal) = trait_list.principal() {
                let poly_trait_ref = principal.with_self_ty(self.tcx, concrete_ty);

                // Walk all methods of the trait, including those of its supertraits
                let entries = self.tcx.vtable_entries(poly_trait_ref);
                let methods = entries.iter().filter_map(|entry| match entry {
                    VtblEntry::MetadataAlign
                    | VtblEntry::MetadataDropInPlace
                    | VtblEntry::MetadataSize
                    | VtblEntry::Vacant => None,
                    VtblEntry::TraitVPtr(_) => {
                        // all super trait items already covered, so skip them.
                        None
                    }
                    VtblEntry::Method(instance) if should_codegen_locally(self.tcx, instance) => {
                        Some(MonoItem::Fn(instance.polymorphize(self.tcx)))
                    }
                    VtblEntry::Method(..) => None,
                });
                trace!(methods=?methods.clone().collect::<Vec<_>>(), "collect_vtable_methods");
                self.collected.extend(methods);
            }
        }

        // Add the destructor for the concrete type.
        let instance = Instance::resolve_drop_in_place(self.tcx, concrete_ty);
        self.collect_instance(instance, false);
    }

    /// Collect an instance depending on how it is used (invoked directly or via fn_ptr).
    fn collect_instance(&mut self, instance: Instance<'tcx>, is_direct_call: bool) {
        let should_collect = match instance.def {
            InstanceDef::Virtual(..) | InstanceDef::Intrinsic(_) => {
                // Instance definition has no body.
                assert!(is_direct_call, "Expected direct call {instance:?}");
                false
            }
            InstanceDef::DropGlue(_, None) => {
                // Only need the glue if we are not calling it directly.
                !is_direct_call
            }
            InstanceDef::CloneShim(..)
            | InstanceDef::ClosureOnceShim { .. }
            | InstanceDef::DropGlue(_, Some(_))
            | InstanceDef::FnPtrShim(..)
            | InstanceDef::Item(..)
            | InstanceDef::ReifyShim(..)
            | InstanceDef::VTableShim(..) => true,
            InstanceDef::ThreadLocalShim(_) | InstanceDef::FnPtrAddrShim(_, _) => true,
        };
        if should_collect && should_codegen_locally(self.tcx, &instance) {
            trace!(?instance, "collect_instance");
            self.collected.insert(MonoItem::Fn(instance.polymorphize(self.tcx)));
        }
    }

    /// Collect constant values represented by static variables.
    fn collect_const_value(&mut self, value: ConstValue<'tcx>) {
        debug!(?value, "collect_const_value");
        match value {
            ConstValue::Scalar(Scalar::Ptr(ptr, _size)) => {
                self.collected.extend(collect_alloc_items(self.tcx, ptr.provenance).iter());
            }
            ConstValue::Slice { data: alloc, start: _, end: _ }
            | ConstValue::ByRef { alloc, .. } => {
                for id in alloc.inner().provenance().provenances() {
                    self.collected.extend(collect_alloc_items(self.tcx, id).iter())
                }
            }
            _ => {}
        }
    }
}

/// Visit every instruction in a function and collect the following:
/// 1. Every function / method / closures that may be directly invoked.
/// 2. Every function / method / closures that may have their address taken.
/// 3. Every method that compose the impl of a trait for a given type when there's a conversion
/// from the type to the trait.
///    - I.e.: If we visit the following code:
///      ```
///      let var = MyType::new();
///      let ptr : &dyn MyTrait = &var;
///      ```
///      We collect the entire implementation of `MyTrait` for `MyType`.
/// 4. Every Static variable that is referenced in the function or constant used in the function.
/// 5. Drop glue.
/// 6. Static Initialization
/// This code has been mostly taken from `rustc_monomorphize::collector::MirNeighborCollector`.
impl<'a, 'tcx> MirVisitor<'tcx> for MonoItemsFnCollector<'a, 'tcx> {
    /// Collect the following:
    /// - Trait implementations when casting from concrete to dyn Trait.
    /// - Functions / Closures that have their address taken.
    /// - Thread Local.
    fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, location: Location) {
        trace!(rvalue=?*rvalue, "visit_rvalue");

        match *rvalue {
            Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::Unsize), ref operand, target) => {
                // Check if the conversion include casting a concrete type to a trait type.
                // If so, collect items from the impl `Trait for Concrete {}`.
                let target_ty = self.monomorphize(target);
                let source_ty = self.monomorphize(operand.ty(self.body, self.tcx));
                let base_coercion =
                    coercion::extract_unsize_casting(self.tcx, source_ty, target_ty);
                if !base_coercion.src_ty.is_trait() && base_coercion.dst_ty.is_trait() {
                    debug!(?base_coercion, "collect_vtable_methods");
                    self.collect_vtable_methods(base_coercion.src_ty, base_coercion.dst_ty);
                }
            }
            Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::ReifyFnPointer), ref operand, _) => {
                let fn_ty = operand.ty(self.body, self.tcx);
                let fn_ty = self.monomorphize(fn_ty);
                if let TyKind::FnDef(def_id, substs) = *fn_ty.kind() {
                    let instance = Instance::resolve_for_fn_ptr(
                        self.tcx,
                        ParamEnv::reveal_all(),
                        def_id,
                        substs,
                    )
                    .unwrap();
                    self.collect_instance(instance, false);
                } else {
                    unreachable!("Expected FnDef type, but got: {:?}", fn_ty);
                }
            }
            Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::ClosureFnPointer(_)), ref operand, _) => {
                let source_ty = operand.ty(self.body, self.tcx);
                let source_ty = self.monomorphize(source_ty);
                match *source_ty.kind() {
                    Closure(def_id, substs) => {
                        let instance = Instance::resolve_closure(
                            self.tcx,
                            def_id,
                            substs,
                            ClosureKind::FnOnce,
                        )
                        .expect("failed to normalize and resolve closure during codegen");
                        self.collect_instance(instance, false);
                    }
                    _ => unreachable!("Unexpected type: {:?}", source_ty),
                }
            }
            Rvalue::ThreadLocalRef(def_id) => {
                assert!(self.tcx.is_thread_local_static(def_id));
                trace!(?def_id, "visit_rvalue thread_local");
                let instance = Instance::mono(self.tcx, def_id);
                if should_codegen_locally(self.tcx, &instance) {
                    trace!("collecting thread-local static {:?}", def_id);
                    self.collected.insert(MonoItem::Static(def_id));
                }
            }
            _ => { /* not interesting */ }
        }

        self.super_rvalue(rvalue, location);
    }

    /// Collect constants that are represented as static variables.
    fn visit_constant(&mut self, constant: &Constant<'tcx>, location: Location) {
        let literal = self.monomorphize(constant.literal);
        debug!(?constant, ?location, ?literal, "visit_constant");
        let val = match literal {
            ConstantKind::Val(const_val, _) => const_val,
            ConstantKind::Ty(ct) => match ct.kind() {
                ConstKind::Value(v) => self.tcx.valtree_to_const_val((ct.ty(), v)),
                ConstKind::Unevaluated(_) => unreachable!(),
                // Nothing to do
                ConstKind::Param(..)
                | ConstKind::Infer(..)
                | ConstKind::Error(..)
                | ConstKind::Expr(..) => return,

                // Shouldn't happen
                ConstKind::Placeholder(..) | ConstKind::Bound(..) => {
                    unreachable!("Unexpected constant type {:?} ({:?})", ct, ct.kind())
                }
            },
            ConstantKind::Unevaluated(un_eval, _) => {
                // Thread local fall into this category.
                match self.tcx.const_eval_resolve(ParamEnv::reveal_all(), un_eval, None) {
                    // The `monomorphize` call should have evaluated that constant already.
                    Ok(const_val) => const_val,
                    Err(ErrorHandled::TooGeneric) => span_bug!(
                        self.body.source_info(location).span,
                        "Unexpected polymorphic constant: {:?}",
                        literal
                    ),
                    Err(error) => {
                        warn!(?error, "Error already reported");
                        return;
                    }
                }
            }
        };
        self.collect_const_value(val);
    }

    /// Collect function calls.
    fn visit_terminator(&mut self, terminator: &Terminator<'tcx>, location: Location) {
        trace!(?terminator, ?location, "visit_terminator");

        let tcx = self.tcx;
        match terminator.kind {
            TerminatorKind::Call { ref func, ref args, .. } => {
                let callee_ty = func.ty(self.body, tcx);
                let fn_ty = self.monomorphize(callee_ty);
                if let TyKind::FnDef(def_id, substs) = *fn_ty.kind() {
                    let instance_opt =
                        Instance::resolve(self.tcx, ParamEnv::reveal_all(), def_id, substs)
                            .unwrap();
                    match instance_opt {
                        None => {
                            let caller = tcx.def_path_str(self.instance.def_id());
                            let callee = tcx.def_path_str(def_id);
                            // Check if the current function has been stubbed.
                            if let Some(stub) = get_stub(tcx, self.instance.def_id()) {
                                // During the MIR stubbing transformation, we do not
                                // force type variables in the stub's signature to
                                // implement the same traits as those in the
                                // original function/method. A trait mismatch shows
                                // up here, when we try to resolve a trait method
                                let generic_ty = args[0].ty(self.body, tcx).peel_refs();
                                let receiver_ty = tcx.subst_and_normalize_erasing_regions(
                                    substs,
                                    ParamEnv::reveal_all(),
                                    EarlyBinder::bind(generic_ty),
                                );
                                let sep = callee.rfind("::").unwrap();
                                let trait_ = &callee[..sep];
                                tcx.sess.span_err(
                                    terminator.source_info.span,
                                    format!(
                                        "`{receiver_ty}` doesn't implement \
                                        `{trait_}`. The function `{caller}` \
                                        cannot be stubbed by `{}` due to \
                                        generic bounds not being met.",
                                        tcx.def_path_str(stub)
                                    ),
                                );
                            } else {
                                panic!("unable to resolve call to `{callee}` in `{caller}`")
                            }
                        }
                        Some(instance) => self.collect_instance(instance, true),
                    };
                } else {
                    assert!(
                        matches!(fn_ty.kind(), TyKind::FnPtr(..)),
                        "Unexpected type: {fn_ty:?}"
                    );
                }
            }
            TerminatorKind::Drop { ref place, .. } => {
                let place_ty = place.ty(self.body, self.tcx).ty;
                let place_mono_ty = self.monomorphize(place_ty);
                let instance = Instance::resolve_drop_in_place(self.tcx, place_mono_ty);
                self.collect_instance(instance, true);
            }
            TerminatorKind::InlineAsm { .. } => {
                // We don't support inline assembly. This shall be replaced by an unsupported
                // construct during codegen.
            }
            TerminatorKind::Terminate { .. } | TerminatorKind::Assert { .. } => {
                // We generate code for this without invoking any lang item.
            }
            TerminatorKind::Goto { .. }
            | TerminatorKind::SwitchInt { .. }
            | TerminatorKind::Resume
            | TerminatorKind::Return
            | TerminatorKind::Unreachable => {}
            TerminatorKind::GeneratorDrop
            | TerminatorKind::Yield { .. }
            | TerminatorKind::FalseEdge { .. }
            | TerminatorKind::FalseUnwind { .. } => {
                unreachable!("Unexpected at this MIR level")
            }
        }

        self.super_terminator(terminator, location);
    }
}

/// Convert a `MonoItem` into a stable `Fingerprint` which can be used as a stable hash across
/// compilation sessions. This allow us to provide a stable deterministic order to codegen.
fn to_fingerprint(tcx: TyCtxt, item: &MonoItem) -> Fingerprint {
    tcx.with_stable_hashing_context(|mut hcx| {
        let mut hasher = StableHasher::new();
        item.hash_stable(&mut hcx, &mut hasher);
        hasher.finish()
    })
}

/// Return whether we should include the item into codegen.
/// - We only skip foreign items.
///
/// Note: Ideally, we should be able to assert that the MIR for non-foreign items are available via
/// call to `tcx.is_mir_available (def_id)`.
/// However, we found an issue where this function was returning `false` for a mutable static
/// item with constant initializer from an upstream crate.
/// See <https://github.com/model-checking/kani/issues/1760> for an example.
fn should_codegen_locally<'tcx>(tcx: TyCtxt<'tcx>, instance: &Instance<'tcx>) -> bool {
    if let Some(def_id) = instance.def.def_id_if_not_guaranteed_local_codegen() {
        // We cannot codegen foreign items.
        !tcx.is_foreign_item(def_id)
    } else {
        // This will include things like VTableShim and other stuff. See the method
        // def_id_if_not_guaranteed_local_codegen for the full list.
        true
    }
}

/// Scans the allocation type and collect static objects.
fn collect_alloc_items(tcx: TyCtxt, alloc_id: AllocId) -> Vec<MonoItem> {
    trace!(alloc=?tcx.global_alloc(alloc_id), ?alloc_id, "collect_alloc_items");
    let mut items = vec![];
    match tcx.global_alloc(alloc_id) {
        GlobalAlloc::Static(def_id) => {
            // This differ from rustc's collector since rustc does not include static from
            // upstream crates.
            assert!(!tcx.is_thread_local_static(def_id));
            let instance = Instance::mono(tcx, def_id);
            should_codegen_locally(tcx, &instance).then(|| items.push(MonoItem::Static(def_id)));
        }
        GlobalAlloc::Function(instance) => {
            should_codegen_locally(tcx, &instance)
                .then(|| items.push(MonoItem::Fn(instance.polymorphize(tcx))));
        }
        GlobalAlloc::Memory(alloc) => {
            items.extend(
                alloc
                    .inner()
                    .provenance()
                    .provenances()
                    .flat_map(|id| collect_alloc_items(tcx, id)),
            );
        }
        GlobalAlloc::VTable(ty, trait_ref) => {
            let vtable_id = tcx.vtable_allocation((ty, trait_ref));
            items.append(&mut collect_alloc_items(tcx, vtable_id));
        }
    };
    items
}

#[cfg(debug_assertions)]
mod debug {
    #![allow(dead_code)]

    use std::{
        collections::{HashMap, HashSet},
        fs::File,
        io::{BufWriter, Write},
    };

    use rustc_session::config::OutputType;

    use super::*;

    #[derive(Debug, Default)]
    pub struct CallGraph<'tcx> {
        // Nodes of the graph.
        nodes: HashSet<Node<'tcx>>,
        edges: HashMap<Node<'tcx>, Vec<Node<'tcx>>>,
        back_edges: HashMap<Node<'tcx>, Vec<Node<'tcx>>>,
    }

    type Node<'tcx> = MonoItem<'tcx>;

    impl<'tcx> CallGraph<'tcx> {
        pub fn add_node(&mut self, item: Node<'tcx>) {
            self.nodes.insert(item);
            self.edges.entry(item).or_default();
            self.back_edges.entry(item).or_default();
        }

        /// Add a new edge "from" -> "to".
        pub fn add_edge(&mut self, from: Node<'tcx>, to: Node<'tcx>) {
            self.add_node(from);
            self.add_node(to);
            self.edges.get_mut(&from).unwrap().push(to);
            self.back_edges.get_mut(&to).unwrap().push(from);
        }

        /// Add multiple new edges for the "from" node.
        pub fn add_edges(&mut self, from: Node<'tcx>, to: &[Node<'tcx>]) {
            self.add_node(from);
            for item in to {
                self.add_edge(from, *item);
            }
        }

        /// Print the graph in DOT format to a file.
        /// See <https://graphviz.org/doc/info/lang.html> for more information.
        pub fn dump_dot(&self, tcx: TyCtxt) -> std::io::Result<()> {
            if let Ok(target) = std::env::var("KANI_REACH_DEBUG") {
                debug!(?target, "dump_dot");
                let outputs = tcx.output_filenames(());
                let path = outputs.output_path(OutputType::Metadata).with_extension("dot");
                let out_file = File::create(&path)?;
                let mut writer = BufWriter::new(out_file);
                writeln!(writer, "digraph ReachabilityGraph {{")?;
                if target.is_empty() {
                    self.dump_all(&mut writer)?;
                } else {
                    // Only dump nodes that led the reachability analysis to the target node.
                    self.dump_reason(&mut writer, &target)?;
                }
                writeln!(writer, "}}")?;
            }

            Ok(())
        }

        /// Write all notes to the given writer.
        fn dump_all<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
            tracing::info!(nodes=?self.nodes.len(), edges=?self.edges.len(), "dump_all");
            for node in &self.nodes {
                writeln!(writer, r#""{node}""#)?;
                for succ in self.edges.get(node).unwrap() {
                    writeln!(writer, r#""{node}" -> "{succ}" "#)?;
                }
            }
            Ok(())
        }

        /// Write all notes that may have led to the discovery of the given target.
        fn dump_reason<W: Write>(&self, writer: &mut W, target: &str) -> std::io::Result<()> {
            let mut queue = self
                .nodes
                .iter()
                .filter(|item| item.to_string().contains(target))
                .collect::<Vec<_>>();
            let mut visited: HashSet<&MonoItem> = HashSet::default();
            tracing::info!(target=?queue, nodes=?self.nodes.len(), edges=?self.edges.len(), "dump_reason");
            while let Some(to_visit) = queue.pop() {
                if !visited.contains(to_visit) {
                    visited.insert(to_visit);
                    queue.extend(self.back_edges.get(to_visit).unwrap());
                }
            }

            for node in &visited {
                writeln!(writer, r#""{node}""#)?;
                for succ in
                    self.edges.get(node).unwrap().iter().filter(|item| visited.contains(item))
                {
                    writeln!(writer, r#""{node}" -> "{succ}" "#)?;
                }
            }
            Ok(())
        }
    }
}
