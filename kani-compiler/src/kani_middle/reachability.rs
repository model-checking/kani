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
use rustc_data_structures::fingerprint::Fingerprint;
use rustc_data_structures::fx::FxHashSet;
use rustc_data_structures::stable_hasher::{HashStable, StableHasher};
use rustc_hir::lang_items::LangItem;
use rustc_middle::mir::interpret::{AllocId, ConstValue, ErrorHandled, GlobalAlloc, Scalar};
use rustc_middle::mir::mono::MonoItem;
use rustc_middle::mir::visit::Visitor as MirVisitor;
use rustc_middle::mir::{
    Body, CastKind, Constant, ConstantKind, Location, Rvalue, Terminator, TerminatorKind,
};
use rustc_middle::span_bug;
use rustc_middle::traits::{ImplSource, ImplSourceUserDefinedData};
use rustc_middle::ty::adjustment::CustomCoerceUnsized;
use rustc_middle::ty::adjustment::PointerCast;
use rustc_middle::ty::TypeAndMut;
use rustc_middle::ty::{
    self, Closure, ClosureKind, ConstKind, Instance, InstanceDef, ParamEnv, TraitRef, Ty, TyCtxt,
    TyKind, TypeFoldable, VtblEntry,
};
use rustc_span::def_id::DefId;
use tracing::{debug, debug_span, trace, warn};

/// Collect all reachable items starting from the given starting points.
pub fn collect_reachable_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    starting_points: &[MonoItem<'tcx>],
) -> Vec<MonoItem<'tcx>> {
    // For each harness, collect items using the same collector.
    // I.e.: This will return any item that is reachable from one or more of the starting points.
    let mut collector = MonoItemsCollector { tcx, collected: FxHashSet::default(), queue: vec![] };
    for item in starting_points {
        collector.collect(*item);
    }

    // Sort the result so code generation follows deterministic order.
    // This helps us to debug the code, but it also provides the user a good experience since the
    // order of the errors and warnings is stable.
    let mut sorted_items: Vec<_> = collector.collected.into_iter().collect();
    sorted_items.sort_by_cached_key(|item| to_fingerprint(tcx, item));
    sorted_items
}

/// Collect all items in the crate that matches the given predicate.
pub fn filter_crate_items<F>(tcx: TyCtxt, predicate: F) -> Vec<MonoItem>
where
    F: FnMut(TyCtxt, DefId) -> bool,
{
    // Filter proof harnesses.
    let mut filter = predicate;
    tcx.hir_crate_items(())
        .items()
        .filter_map(|hir_id| {
            let def_id = hir_id.def_id.to_def_id();
            filter(tcx, def_id).then(|| MonoItem::Fn(Instance::mono(tcx, def_id)))
        })
        .collect()
}

struct MonoItemsCollector<'tcx> {
    /// The compiler context.
    tcx: TyCtxt<'tcx>,
    /// Set of collected items used to avoid entering recursion loops.
    collected: FxHashSet<MonoItem<'tcx>>,
    /// Items enqueued for visiting.
    queue: Vec<MonoItem<'tcx>>,
}

impl<'tcx> MonoItemsCollector<'tcx> {
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
                match to_visit {
                    MonoItem::Fn(instance) => {
                        self.visit_fn(instance);
                    }
                    MonoItem::Static(def_id) => {
                        self.visit_static(def_id);
                    }
                    MonoItem::GlobalAsm(_) => {
                        self.visit_asm(to_visit);
                    }
                }
            }
        }
    }

    /// Visit a function and collect all mono-items reachable from its instructions.
    fn visit_fn(&mut self, instance: Instance<'tcx>) {
        let _guard = debug_span!("visit_fn", function=?instance).entered();
        let body = self.tcx.instance_mir(instance.def);
        let mut collector =
            MonoItemsFnCollector { tcx: self.tcx, collected: FxHashSet::default(), instance, body };
        collector.visit_body(body);
        self.queue.extend(collector.collected.iter().filter(|item| !self.collected.contains(item)));
    }

    /// Visit a static object and collect drop / initialization functions.
    fn visit_static(&mut self, def_id: DefId) {
        let _guard = debug_span!("visit_static", ?def_id).entered();
        let instance = Instance::mono(self.tcx, def_id);

        // Collect drop function.
        let static_ty = instance.ty(self.tcx, ParamEnv::reveal_all());
        let instance = Instance::resolve_drop_in_place(self.tcx, static_ty);
        self.queue.push(MonoItem::Fn(instance.polymorphize(self.tcx)));

        // Collect initialization.
        let alloc = self.tcx.eval_static_initializer(def_id).unwrap();
        for &id in alloc.inner().provenance().values() {
            self.queue.extend(collect_alloc_items(self.tcx, id).iter());
        }
    }

    /// Visit global assembly and collect its item.
    fn visit_asm(&mut self, item: MonoItem<'tcx>) {
        debug!(?item, "visit_asm");
        self.collected.insert(item);
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
        T: TypeFoldable<'tcx>,
    {
        trace!(instance=?self.instance, ?value, "monomorphize");
        self.instance.subst_mir_and_normalize_erasing_regions(
            self.tcx,
            ParamEnv::reveal_all(),
            value,
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
                for &id in alloc.inner().provenance().values() {
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
            Rvalue::Cast(CastKind::Pointer(PointerCast::Unsize), ref operand, target) => {
                // Check if the conversion include casting a concrete type to a trait type.
                // If so, collect items from the impl `Trait for Concrete {}`.
                let target_ty = self.monomorphize(target);
                let source_ty = self.monomorphize(operand.ty(self.body, self.tcx));
                let (src_inner, dst_inner) = extract_trait_casting(self.tcx, source_ty, target_ty);
                if !src_inner.is_trait() && dst_inner.is_trait() {
                    debug!(concrete_ty=?src_inner, trait_ty=?dst_inner, "collect_vtable_methods");
                    self.collect_vtable_methods(src_inner, dst_inner);
                }
            }
            Rvalue::Cast(CastKind::Pointer(PointerCast::ReifyFnPointer), ref operand, _) => {
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
            Rvalue::Cast(CastKind::Pointer(PointerCast::ClosureFnPointer(_)), ref operand, _) => {
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
                ConstKind::Param(..) | ConstKind::Infer(..) | ConstKind::Error(..) => return,

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
            TerminatorKind::Call { ref func, .. } => {
                let callee_ty = func.ty(self.body, tcx);
                let fn_ty = self.monomorphize(callee_ty);
                if let TyKind::FnDef(def_id, substs) = *fn_ty.kind() {
                    let instance =
                        Instance::resolve(self.tcx, ParamEnv::reveal_all(), def_id, substs)
                            .unwrap()
                            .unwrap();
                    self.collect_instance(instance, true);
                } else {
                    assert!(
                        matches!(fn_ty.kind(), TyKind::FnPtr(..)),
                        "Unexpected type: {:?}",
                        fn_ty
                    );
                }
            }
            TerminatorKind::Drop { ref place, .. }
            | TerminatorKind::DropAndReplace { ref place, .. } => {
                let place_ty = place.ty(self.body, self.tcx).ty;
                let place_mono_ty = self.monomorphize(place_ty);
                let instance = Instance::resolve_drop_in_place(self.tcx, place_mono_ty);
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

/// Extract the pair (`T`, `U`) for a unsized coercion where type `T` implements `Unsize<U>`.
/// I.e., `U` is either a trait or a slice.
/// For more details, please refer to:
/// <https://doc.rust-lang.org/reference/type-coercions.html#unsized-coercions>
///
/// This is used to determine the vtable implementation that must be tracked by the fat pointer.
///
/// For example, if `&u8` is being converted to `&dyn Debug`, this method would return:
/// `(u8, dyn Debug)`.
///
/// There are a few interesting cases (references / pointers are handled the same way):
/// 1. Coercion between `&T` to `&U`.
///    - This is the base case.
///    - In this case, we extract the type that is pointed to.
/// 2. Coercion between smart pointers like `Rc<T>` to `Rc<U>`.
///    - Smart pointers implement the `CoerceUnsize` trait.
///    - Use CustomCoerceUnsized information to traverse the smart pointer structure and find the
///      underlying pointer.
///    - Use base case to extract `T` and `U`.
/// 3. Coercion between `&Wrapper<T>` to `&Wrapper<U>`.
///    - Apply base case to extract the pair `(Wrapper<T>, Wrapper<U>)`.
///    - Extract the tail element of the struct which are of type `T` and `U`, respectively.
/// 4. Coercion between smart pointers of wrapper structs.
///    - Apply the logic from item 2 then item 3.
fn extract_trait_casting<'tcx>(
    tcx: TyCtxt<'tcx>,
    src_ty: Ty<'tcx>,
    dst_ty: Ty<'tcx>,
) -> (Ty<'tcx>, Ty<'tcx>) {
    trace!(?dst_ty, ?src_ty, "find_trait_conversion");
    let mut src_inner_ty = src_ty;
    let mut dst_inner_ty = dst_ty;
    (src_inner_ty, dst_inner_ty) = loop {
        // Extract the pointee types from pointers (including smart pointers) that form the base of
        // the conversion.
        match (&src_inner_ty.kind(), &dst_inner_ty.kind()) {
            (&ty::Adt(src_def, src_substs), &ty::Adt(dst_def, dst_substs))
                if !src_def.is_box() || !dst_def.is_box() =>
            {
                // Handle smart pointers by using CustomCoerceUnsized to find the field being
                // coerced.
                assert_eq!(src_def, dst_def);
                let src_fields = &src_def.non_enum_variant().fields;
                let dst_fields = &dst_def.non_enum_variant().fields;
                assert_eq!(src_fields.len(), dst_fields.len());

                let CustomCoerceUnsized::Struct(coerce_index) =
                    custom_coerce_unsize_info(tcx, src_inner_ty, dst_inner_ty);
                assert!(coerce_index < src_fields.len());

                src_inner_ty = src_fields[coerce_index].ty(tcx, src_substs);
                dst_inner_ty = dst_fields[coerce_index].ty(tcx, dst_substs);
            }
            _ => {
                // Base case is always a pointer (Box, raw_pointer or reference).
                let src_pointee = extract_pointee(src_inner_ty).expect(&format!(
                    "Expected source to be a pointer. Found {:?} instead",
                    src_inner_ty
                ));
                let dst_pointee = extract_pointee(dst_inner_ty).expect(&format!(
                    "Expected destination to be a pointer. Found {:?} instead",
                    dst_inner_ty
                ));
                break (src_pointee, dst_pointee);
            }
        }
    };

    tcx.struct_lockstep_tails_erasing_lifetimes(src_inner_ty, dst_inner_ty, ParamEnv::reveal_all())
}

/// Extract pointee type from builtin pointer types.
fn extract_pointee(typ: Ty) -> Option<Ty> {
    typ.builtin_deref(true).map(|TypeAndMut { ty, .. }| ty)
}

/// Get information about an unsized coercion.
/// This code was extracted from `rustc_monomorphize` crate.
/// <https://github.com/rust-lang/rust/blob/4891d57f7aab37b5d6a84f2901c0bb8903111d53/compiler/rustc_monomorphize/src/lib.rs#L25-L46>
fn custom_coerce_unsize_info<'tcx>(
    tcx: TyCtxt<'tcx>,
    source_ty: Ty<'tcx>,
    target_ty: Ty<'tcx>,
) -> CustomCoerceUnsized {
    let def_id = tcx.require_lang_item(LangItem::CoerceUnsized, None);

    let trait_ref = ty::Binder::dummy(TraitRef {
        def_id,
        substs: tcx.mk_substs_trait(source_ty, &[target_ty.into()]),
    });

    match tcx.codegen_select_candidate((ParamEnv::reveal_all(), trait_ref)) {
        Ok(ImplSource::UserDefined(ImplSourceUserDefinedData { impl_def_id, .. })) => {
            tcx.coerce_unsized_info(impl_def_id).custom_kind.unwrap()
        }
        impl_source => {
            unreachable!("invalid `CoerceUnsized` impl_source: {:?}", impl_source);
        }
    }
}

/// Scans the allocation type and collect static objects.
fn collect_alloc_items<'tcx>(tcx: TyCtxt<'tcx>, alloc_id: AllocId) -> Vec<MonoItem> {
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
                alloc.inner().provenance().values().flat_map(|id| collect_alloc_items(tcx, *id)),
            );
        }
        GlobalAlloc::VTable(ty, trait_ref) => {
            let vtable_id = tcx.vtable_allocation((ty, trait_ref));
            items.append(&mut collect_alloc_items(tcx, vtable_id));
        }
    };
    items
}
