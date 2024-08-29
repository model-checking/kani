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
//!
//! Note that this is a copy of `reachability.rs` that uses StableMIR but the public APIs are still
//! kept with internal APIs.
use tracing::{debug, debug_span, trace};

use rustc_data_structures::fingerprint::Fingerprint;
use rustc_data_structures::fx::FxHashSet;
use rustc_data_structures::stable_hasher::{HashStable, StableHasher};
use rustc_middle::ty::{TyCtxt, VtblEntry};
use rustc_session::config::OutputType;
use rustc_smir::rustc_internal;
use stable_mir::mir::alloc::{AllocId, GlobalAlloc};
use stable_mir::mir::mono::{Instance, InstanceKind, MonoItem, StaticDef};
use stable_mir::mir::{
    visit::Location, Body, CastKind, ConstOperand, MirVisitor, PointerCoercion, Rvalue, Terminator,
    TerminatorKind,
};
use stable_mir::ty::{Allocation, ClosureKind, ConstantKind, RigidTy, Ty, TyKind};
use stable_mir::CrateItem;
use stable_mir::{CrateDef, ItemKind};
use std::fmt::{Display, Formatter};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufWriter, Write},
};

use crate::kani_middle::coercion;
use crate::kani_middle::coercion::CoercionBase;
use crate::kani_middle::transform::BodyTransformation;

/// Collect all reachable items starting from the given starting points.
pub fn collect_reachable_items(
    tcx: TyCtxt,
    transformer: &mut BodyTransformation,
    starting_points: &[MonoItem],
) -> (Vec<MonoItem>, CallGraph) {
    // For each harness, collect items using the same collector.
    // I.e.: This will return any item that is reachable from one or more of the starting points.
    let mut collector = MonoItemsCollector::new(tcx, transformer);
    for item in starting_points {
        collector.collect(item.clone());
    }

    #[cfg(debug_assertions)]
    collector
        .call_graph
        .dump_dot(tcx)
        .unwrap_or_else(|e| tracing::error!("Failed to dump call graph: {e}"));

    tcx.dcx().abort_if_errors();
    // Sort the result so code generation follows deterministic order.
    // This helps us to debug the code, but it also provides the user a good experience since the
    // order of the errors and warnings is stable.
    let mut sorted_items: Vec<_> = collector.collected.into_iter().collect();
    sorted_items.sort_by_cached_key(|item| to_fingerprint(tcx, item));
    (sorted_items, collector.call_graph)
}

/// Collect all (top-level) items in the crate that matches the given predicate.
/// An item can only be a root if they are a non-generic function.
pub fn filter_crate_items<F>(tcx: TyCtxt, predicate: F) -> Vec<Instance>
where
    F: Fn(TyCtxt, Instance) -> bool,
{
    let crate_items = stable_mir::all_local_items();
    // Filter regular items.
    crate_items
        .iter()
        .filter_map(|item| {
            // Only collect monomorphic items.
            // TODO: Remove the def_kind check once https://github.com/rust-lang/rust/pull/119135 has been released.
            let def_id = rustc_internal::internal(tcx, item.def_id());
            (matches!(tcx.def_kind(def_id), rustc_hir::def::DefKind::Ctor(..))
                || matches!(item.kind(), ItemKind::Fn))
            .then(|| {
                Instance::try_from(*item)
                    .ok()
                    .and_then(|instance| predicate(tcx, instance).then_some(instance))
            })
            .flatten()
        })
        .collect::<Vec<_>>()
}

/// Use a predicate to find `const` declarations, then extract all items reachable from them.
///
/// Probably only specifically useful with a predicate to find `TestDescAndFn` const declarations from
/// tests and extract the closures from them.
pub fn filter_const_crate_items<F>(
    tcx: TyCtxt,
    transformer: &mut BodyTransformation,
    mut predicate: F,
) -> Vec<MonoItem>
where
    F: FnMut(TyCtxt, Instance) -> bool,
{
    let crate_items = stable_mir::all_local_items();
    let mut roots = Vec::new();
    // Filter regular items.
    for item in crate_items {
        // Only collect monomorphic items.
        if let Ok(instance) = Instance::try_from(item) {
            if predicate(tcx, instance) {
                let body = transformer.body(tcx, instance);
                let mut collector =
                    MonoItemsFnCollector { tcx, body: &body, collected: FxHashSet::default() };
                collector.visit_body(&body);
                roots.extend(collector.collected.into_iter());
            }
        }
    }
    roots.into_iter().map(|root| root.item).collect()
}

/// Reason for introducing an edge in the call graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum CollectionReason {
    DirectCall,
    IndirectCall,
    VTableMethod,
    Static,
    StaticDrop,
}

/// A destination of the edge in the call graph.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct CollectedItem {
    item: MonoItem,
    reason: CollectionReason,
}

struct MonoItemsCollector<'tcx, 'a> {
    /// The compiler context.
    tcx: TyCtxt<'tcx>,
    /// The body transformation object used to retrieve a transformed body.
    transformer: &'a mut BodyTransformation,
    /// Set of collected items used to avoid entering recursion loops.
    collected: FxHashSet<MonoItem>,
    /// Items enqueued for visiting.
    queue: Vec<MonoItem>,
    /// Call graph used for dataflow analysis.
    call_graph: CallGraph,
}

impl<'tcx, 'a> MonoItemsCollector<'tcx, 'a> {
    pub fn new(tcx: TyCtxt<'tcx>, transformer: &'a mut BodyTransformation) -> Self {
        MonoItemsCollector {
            tcx,
            collected: FxHashSet::default(),
            queue: vec![],
            call_graph: CallGraph::default(),
            transformer,
        }
    }

    /// Collects all reachable items starting from the given root.
    pub fn collect(&mut self, root: MonoItem) {
        debug!(?root, "collect");
        self.queue.push(root);
        self.reachable_items();
    }

    /// Traverses the call graph starting from the given root. For every function, we visit all
    /// instruction looking for the items that should be included in the compilation.
    fn reachable_items(&mut self) {
        while let Some(to_visit) = self.queue.pop() {
            if !self.collected.contains(&to_visit) {
                self.collected.insert(to_visit.clone());
                let next_items = match &to_visit {
                    MonoItem::Fn(instance) => self.visit_fn(*instance),
                    MonoItem::Static(static_def) => self.visit_static(*static_def),
                    MonoItem::GlobalAsm(_) => {
                        self.visit_asm(&to_visit);
                        vec![]
                    }
                };
                self.call_graph.add_edges(to_visit, &next_items);

                self.queue.extend(next_items.into_iter().filter_map(
                    |CollectedItem { item, .. }| (!self.collected.contains(&item)).then_some(item),
                ));
            }
        }
    }

    /// Visit a function and collect all mono-items reachable from its instructions.
    fn visit_fn(&mut self, instance: Instance) -> Vec<CollectedItem> {
        let _guard = debug_span!("visit_fn", function=?instance).entered();
        let body = self.transformer.body(self.tcx, instance);
        let mut collector =
            MonoItemsFnCollector { tcx: self.tcx, collected: FxHashSet::default(), body: &body };
        collector.visit_body(&body);
        collector.collected.into_iter().collect()
    }

    /// Visit a static object and collect drop / initialization functions.
    fn visit_static(&mut self, def: StaticDef) -> Vec<CollectedItem> {
        let _guard = debug_span!("visit_static", ?def).entered();
        let mut next_items = vec![];

        // Collect drop function.
        let static_ty = def.ty();
        let instance = Instance::resolve_drop_in_place(static_ty);
        next_items
            .push(CollectedItem { item: instance.into(), reason: CollectionReason::StaticDrop });

        // Collect initialization.
        let alloc = def.eval_initializer().unwrap();
        for (_, prov) in alloc.provenance.ptrs {
            next_items.extend(
                collect_alloc_items(prov.0)
                    .into_iter()
                    .map(|item| CollectedItem { item, reason: CollectionReason::Static }),
            );
        }

        next_items
    }

    /// Visit global assembly and collect its item.
    fn visit_asm(&mut self, item: &MonoItem) {
        debug!(?item, "visit_asm");
    }
}

struct MonoItemsFnCollector<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    collected: FxHashSet<CollectedItem>,
    body: &'a Body,
}

impl<'a, 'tcx> MonoItemsFnCollector<'a, 'tcx> {
    /// Collect the implementation of all trait methods and its supertrait methods for the given
    /// concrete type.
    fn collect_vtable_methods(&mut self, concrete_ty: Ty, trait_ty: Ty) {
        trace!(?concrete_ty, ?trait_ty, "collect_vtable_methods");
        let concrete_kind = concrete_ty.kind();
        let trait_kind = trait_ty.kind();

        assert!(!concrete_kind.is_trait(), "expected a concrete type, but found `{concrete_ty:?}`");
        assert!(trait_kind.is_trait(), "expected a trait `{trait_ty:?}`");
        if let Some(principal) = trait_kind.trait_principal() {
            // A trait object type can have multiple trait bounds but up to one non-auto-trait
            // bound. This non-auto-trait, named principal, is the only one that can have methods.
            // https://doc.rust-lang.org/reference/special-types-and-traits.html#auto-traits
            let poly_trait_ref = principal.with_self_ty(concrete_ty);

            // Walk all methods of the trait, including those of its supertraits
            let entries =
                self.tcx.vtable_entries(rustc_internal::internal(self.tcx, &poly_trait_ref));
            let methods = entries.iter().filter_map(|entry| match entry {
                VtblEntry::MetadataAlign
                | VtblEntry::MetadataDropInPlace
                | VtblEntry::MetadataSize
                | VtblEntry::Vacant => None,
                VtblEntry::TraitVPtr(_) => {
                    // all super trait items already covered, so skip them.
                    None
                }
                VtblEntry::Method(instance) => {
                    let instance = rustc_internal::stable(instance);
                    should_codegen_locally(&instance).then_some(MonoItem::Fn(instance))
                }
            });
            trace!(methods=?methods.clone().collect::<Vec<_>>(), "collect_vtable_methods");
            self.collected.extend(
                methods.map(|item| CollectedItem { item, reason: CollectionReason::VTableMethod }),
            );
        }

        // Add the destructor for the concrete type.
        let instance = Instance::resolve_drop_in_place(concrete_ty);
        self.collect_instance(instance, false);
    }

    /// Collect an instance depending on how it is used (invoked directly or via fn_ptr).
    fn collect_instance(&mut self, instance: Instance, is_direct_call: bool) {
        let should_collect = match instance.kind {
            InstanceKind::Virtual { .. } => {
                // Instance definition has no body.
                assert!(is_direct_call, "Expected direct call {instance:?}");
                false
            }
            InstanceKind::Intrinsic => {
                // Intrinsics may have a fallback body.
                assert!(is_direct_call, "Expected direct call {instance:?}");
                let TyKind::RigidTy(RigidTy::FnDef(def, _)) = instance.ty().kind() else {
                    unreachable!("Expected function type for intrinsic: {instance:?}")
                };
                // The compiler is currently transitioning how to handle intrinsic fallback body.
                // Until https://github.com/rust-lang/project-stable-mir/issues/79 is implemented
                // we have to check `must_be_overridden` and `has_body`.
                !def.as_intrinsic().unwrap().must_be_overridden() && instance.has_body()
            }
            InstanceKind::Shim | InstanceKind::Item => true,
        };
        if should_collect && should_codegen_locally(&instance) {
            trace!(?instance, "collect_instance");
            let reason = if is_direct_call {
                CollectionReason::DirectCall
            } else {
                CollectionReason::IndirectCall
            };
            self.collected.insert(CollectedItem { item: instance.into(), reason });
        }
    }

    /// Collect constant values represented by static variables.
    fn collect_allocation(&mut self, alloc: &Allocation) {
        debug!(?alloc, "collect_allocation");
        for (_, id) in &alloc.provenance.ptrs {
            self.collected.extend(
                collect_alloc_items(id.0)
                    .into_iter()
                    .map(|item| CollectedItem { item, reason: CollectionReason::Static }),
            )
        }
    }
}

/// Visit every instruction in a function and collect the following:
/// 1. Every function / method / closures that may be directly invoked.
/// 2. Every function / method / closures that may have their address taken.
/// 3. Every method that compose the impl of a trait for a given type when there's a conversion
///    from the type to the trait.
///    - I.e.: If we visit the following code:
///      ```
///      let var = MyType::new();
///      let ptr : &dyn MyTrait = &var;
///      ```
///      We collect the entire implementation of `MyTrait` for `MyType`.
/// 4. Every Static variable that is referenced in the function or constant used in the function.
/// 5. Drop glue.
/// 6. Static Initialization
///
/// Remark: This code has been mostly taken from `rustc_monomorphize::collector::MirNeighborCollector`.
impl<'a, 'tcx> MirVisitor for MonoItemsFnCollector<'a, 'tcx> {
    /// Collect the following:
    /// - Trait implementations when casting from concrete to dyn Trait.
    /// - Functions / Closures that have their address taken.
    /// - Thread Local.
    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: Location) {
        trace!(rvalue=?*rvalue, "visit_rvalue");

        match *rvalue {
            Rvalue::Cast(
                CastKind::PointerCoercion(PointerCoercion::Unsize),
                ref operand,
                target,
            ) => {
                // Check if the conversion include casting a concrete type to a trait type.
                // If so, collect items from the impl `Trait for Concrete {}`.
                let target_ty = target;
                let source_ty = operand.ty(self.body.locals()).unwrap();
                let (src_ty, dst_ty) = extract_unsize_coercion(self.tcx, source_ty, target_ty);
                if !src_ty.kind().is_trait() && dst_ty.kind().is_trait() {
                    debug!(?src_ty, ?dst_ty, "collect_vtable_methods");
                    self.collect_vtable_methods(src_ty, dst_ty);
                }
            }
            Rvalue::Cast(
                CastKind::PointerCoercion(PointerCoercion::ReifyFnPointer),
                ref operand,
                _,
            ) => {
                let fn_kind = operand.ty(self.body.locals()).unwrap().kind();
                if let RigidTy::FnDef(fn_def, args) = fn_kind.rigid().unwrap() {
                    let instance = Instance::resolve_for_fn_ptr(*fn_def, args).unwrap();
                    self.collect_instance(instance, false);
                } else {
                    unreachable!("Expected FnDef type, but got: {:?}", fn_kind);
                }
            }
            Rvalue::Cast(
                CastKind::PointerCoercion(PointerCoercion::ClosureFnPointer(_)),
                ref operand,
                _,
            ) => {
                let source_ty = operand.ty(self.body.locals()).unwrap();
                match source_ty.kind().rigid().unwrap() {
                    RigidTy::Closure(def_id, args) => {
                        let instance =
                            Instance::resolve_closure(*def_id, args, ClosureKind::FnOnce)
                                .expect("failed to normalize and resolve closure during codegen");
                        self.collect_instance(instance, false);
                    }
                    _ => unreachable!("Unexpected type: {:?}", source_ty),
                }
            }
            Rvalue::ThreadLocalRef(item) => {
                trace!(?item, "visit_rvalue thread_local");
                let item = MonoItem::Static(StaticDef::try_from(item).unwrap());
                self.collected.insert(CollectedItem { item, reason: CollectionReason::Static });
            }
            _ => { /* not interesting */ }
        }

        self.super_rvalue(rvalue, location);
    }

    /// Collect constants that are represented as static variables.
    fn visit_const_operand(&mut self, constant: &ConstOperand, location: Location) {
        debug!(?constant, ?location, literal=?constant.const_, "visit_constant");
        let allocation = match constant.const_.kind() {
            ConstantKind::Allocated(allocation) => allocation,
            ConstantKind::Unevaluated(_) => {
                unreachable!("Instance with polymorphic constant: `{constant:?}`")
            }
            ConstantKind::Param(_) => unreachable!("Unexpected parameter constant: {constant:?}"),
            ConstantKind::ZeroSized => {
                // Nothing to do here.
                return;
            }
            ConstantKind::Ty(_) => {
                // Nothing to do here.
                return;
            }
        };
        self.collect_allocation(&allocation);
    }

    /// Collect function calls.
    fn visit_terminator(&mut self, terminator: &Terminator, location: Location) {
        trace!(?terminator, ?location, "visit_terminator");

        match terminator.kind {
            TerminatorKind::Call { ref func, .. } => {
                let fn_ty = func.ty(self.body.locals()).unwrap();
                if let TyKind::RigidTy(RigidTy::FnDef(fn_def, args)) = fn_ty.kind() {
                    let instance = Instance::resolve(fn_def, &args).unwrap();
                    self.collect_instance(instance, true);
                } else {
                    assert!(
                        matches!(fn_ty.kind().rigid(), Some(RigidTy::FnPtr(..))),
                        "Unexpected type: {fn_ty:?}"
                    );
                }
            }
            TerminatorKind::Drop { ref place, .. } => {
                let place_ty = place.ty(self.body.locals()).unwrap();
                let instance = Instance::resolve_drop_in_place(place_ty);
                self.collect_instance(instance, true);
            }
            TerminatorKind::InlineAsm { .. } => {
                // We don't support inline assembly. This shall be replaced by an unsupported
                // construct during codegen.
            }
            TerminatorKind::Abort { .. } | TerminatorKind::Assert { .. } => {
                // We generate code for this without invoking any lang item.
            }
            TerminatorKind::Goto { .. }
            | TerminatorKind::SwitchInt { .. }
            | TerminatorKind::Resume
            | TerminatorKind::Return
            | TerminatorKind::Unreachable => {}
        }

        self.super_terminator(terminator, location);
    }
}

fn extract_unsize_coercion(tcx: TyCtxt, orig_ty: Ty, dst_trait: Ty) -> (Ty, Ty) {
    let CoercionBase { src_ty, dst_ty } = coercion::extract_unsize_casting(
        tcx,
        rustc_internal::internal(tcx, orig_ty),
        rustc_internal::internal(tcx, dst_trait),
    );
    (rustc_internal::stable(src_ty), rustc_internal::stable(dst_ty))
}

/// Convert a `MonoItem` into a stable `Fingerprint` which can be used as a stable hash across
/// compilation sessions. This allow us to provide a stable deterministic order to codegen.
fn to_fingerprint(tcx: TyCtxt, item: &MonoItem) -> Fingerprint {
    tcx.with_stable_hashing_context(|mut hcx| {
        let mut hasher = StableHasher::new();
        rustc_internal::internal(tcx, item).hash_stable(&mut hcx, &mut hasher);
        hasher.finish()
    })
}

/// Return whether we should include the item into codegen.
fn should_codegen_locally(instance: &Instance) -> bool {
    !instance.is_foreign_item()
}

fn collect_alloc_items(alloc_id: AllocId) -> Vec<MonoItem> {
    trace!(?alloc_id, "collect_alloc_items");
    let mut items = vec![];
    match GlobalAlloc::from(alloc_id) {
        GlobalAlloc::Static(def) => {
            // This differ from rustc's collector since rustc does not include static from
            // upstream crates.
            let instance = Instance::try_from(CrateItem::from(def)).unwrap();
            should_codegen_locally(&instance).then(|| items.push(MonoItem::from(def)));
        }
        GlobalAlloc::Function(instance) => {
            should_codegen_locally(&instance).then(|| items.push(MonoItem::from(instance)));
        }
        GlobalAlloc::Memory(alloc) => {
            items.extend(
                alloc.provenance.ptrs.iter().flat_map(|(_, prov)| collect_alloc_items(prov.0)),
            );
        }
        vtable_alloc @ GlobalAlloc::VTable(..) => {
            let vtable_id = vtable_alloc.vtable_allocation().unwrap();
            items = collect_alloc_items(vtable_id);
        }
    };
    items
}

/// Call graph with edges annotated with the reason why they were added to the graph.
#[derive(Debug, Default)]
pub struct CallGraph {
    /// Nodes of the graph.
    nodes: HashSet<Node>,
    /// Edges of the graph.
    edges: HashMap<Node, Vec<CollectedNode>>,
    /// Since the graph is directed, we also store back edges.
    back_edges: HashMap<Node, Vec<CollectedNode>>,
}

/// Newtype around MonoItem.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct Node(pub MonoItem);

/// Newtype around CollectedItem.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct CollectedNode(pub CollectedItem);

impl CallGraph {
    /// Add a new node into a graph.
    fn add_node(&mut self, item: MonoItem) {
        let node = Node(item);
        self.nodes.insert(node.clone());
        self.edges.entry(node.clone()).or_default();
        self.back_edges.entry(node).or_default();
    }

    /// Add a new edge "from" -> "to".
    fn add_edge(&mut self, from: MonoItem, to: MonoItem, collection_reason: CollectionReason) {
        let from_node = Node(from.clone());
        let to_node = Node(to.clone());
        self.add_node(from.clone());
        self.add_node(to.clone());
        self.edges
            .get_mut(&from_node)
            .unwrap()
            .push(CollectedNode(CollectedItem { item: to, reason: collection_reason }));
        self.back_edges
            .get_mut(&to_node)
            .unwrap()
            .push(CollectedNode(CollectedItem { item: from, reason: collection_reason }));
    }

    /// Add multiple new edges for the "from" node.
    fn add_edges(&mut self, from: MonoItem, to: &[CollectedItem]) {
        self.add_node(from.clone());
        for CollectedItem { item, reason } in to {
            self.add_edge(from.clone(), item.clone(), *reason);
        }
    }

    /// Print the graph in DOT format to a file.
    /// See <https://graphviz.org/doc/info/lang.html> for more information.
    #[allow(unused)]
    fn dump_dot(&self, tcx: TyCtxt) -> std::io::Result<()> {
        if let Ok(target) = std::env::var("KANI_REACH_DEBUG") {
            debug!(?target, "dump_dot");
            let outputs = tcx.output_filenames(());
            let base_path = outputs.path(OutputType::Metadata);
            let path = base_path.as_path().with_extension("dot");
            let out_file = File::create(path)?;
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
    #[allow(unused)]
    fn dump_all<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        tracing::info!(nodes=?self.nodes.len(), edges=?self.edges.len(), "dump_all");
        for node in &self.nodes {
            writeln!(writer, r#""{node}""#)?;
            for succ in self.edges.get(node).unwrap() {
                let reason = succ.0.reason;
                writeln!(writer, r#""{node}" -> "{succ}" [label={reason:?}] "#)?;
            }
        }
        Ok(())
    }

    /// Write all notes that may have led to the discovery of the given target.
    #[allow(unused)]
    fn dump_reason<W: Write>(&self, writer: &mut W, target: &str) -> std::io::Result<()> {
        let mut queue: Vec<Node> =
            self.nodes.iter().filter(|item| item.to_string().contains(target)).cloned().collect();
        let mut visited: HashSet<Node> = HashSet::default();
        tracing::info!(target=?queue, nodes=?self.nodes.len(), edges=?self.edges.len(), "dump_reason");
        while let Some(to_visit) = queue.pop() {
            if !visited.contains(&to_visit) {
                visited.insert(to_visit.clone());
                queue.extend(
                    self.back_edges
                        .get(&to_visit)
                        .unwrap()
                        .iter()
                        .map(|item| Node::from(item.clone())),
                );
            }
        }

        for node in &visited {
            writeln!(writer, r#""{node}""#)?;
            let edges = self.edges.get(node).unwrap();
            for succ in edges.iter().filter(|item| {
                let node = Node::from((*item).clone());
                visited.contains(&node)
            }) {
                let reason = succ.0.reason;
                writeln!(writer, r#""{node}" -> "{succ}" [label={reason:?}] "#)?;
            }
        }
        Ok(())
    }

    /// Get all items adjacent to the current item in the
    /// call graph.
    pub fn adjacencies(&self, node: MonoItem) -> Vec<&MonoItem> {
        match self.edges.get(&Node(node)) {
            None => vec![],
            Some(list) => list.iter().map(|collected| &collected.0.item).collect(),
        }
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            MonoItem::Fn(instance) => write!(f, "{}", instance.name()),
            MonoItem::Static(def) => write!(f, "{}", def.name()),
            MonoItem::GlobalAsm(asm) => write!(f, "{asm:?}"),
        }
    }
}

impl Display for CollectedNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0.item {
            MonoItem::Fn(instance) => write!(f, "{}", instance.name()),
            MonoItem::Static(def) => write!(f, "{}", def.name()),
            MonoItem::GlobalAsm(asm) => write!(f, "{asm:?}"),
        }
    }
}

impl From<CollectedNode> for Node {
    fn from(value: CollectedNode) -> Self {
        Node(value.0.item)
    }
}
