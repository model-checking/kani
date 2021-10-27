#![doc(html_root_url = "https://doc.rust-lang.org/nightly/nightly-rustc/")]
#![feature(in_band_lifetimes)]
#![feature(nll)]
#![feature(control_flow_enum)]
#![feature(try_blocks)]
#![feature(associated_type_defaults)]
#![recursion_limit = "256"]
#![cfg_attr(not(bootstrap), allow(rustc::potential_query_instability))]

use rustc_ast::MacroDef;
use rustc_attr as attr;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::struct_span_err;
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId, LocalDefIdSet};
use rustc_hir::def_id::{CRATE_DEF_ID, CRATE_DEF_INDEX, LOCAL_CRATE};
use rustc_hir::intravisit::{self, DeepVisitor, NestedVisitorMap, Visitor};
use rustc_hir::{AssocItemKind, HirIdSet, Node, PatKind};
use rustc_middle::bug;
use rustc_middle::hir::map::Map;
use rustc_middle::middle::privacy::{AccessLevel, AccessLevels};
use rustc_middle::span_bug;
use rustc_middle::thir::abstract_const::Node as ACNode;
use rustc_middle::ty::fold::TypeVisitor;
use rustc_middle::ty::query::Providers;
use rustc_middle::ty::subst::InternalSubsts;
use rustc_middle::ty::{self, Const, GenericParamDefKind, TraitRef, Ty, TyCtxt, TypeFoldable};
use rustc_session::lint;
use rustc_span::hygiene::Transparency;
use rustc_span::symbol::{kw, sym, Ident};
use rustc_span::Span;
use rustc_trait_selection::traits::const_evaluatable::{self, AbstractConst};

use std::marker::PhantomData;
use std::ops::ControlFlow;
use std::{cmp, fmt, mem};

////////////////////////////////////////////////////////////////////////////////
/// Generic infrastructure used to implement specific visitors below.
////////////////////////////////////////////////////////////////////////////////

/// Implemented to visit all `DefId`s in a type.
/// Visiting `DefId`s is useful because visibilities and reachabilities are attached to them.
/// The idea is to visit "all components of a type", as documented in
/// <https://github.com/rust-lang/rfcs/blob/master/text/2145-type-privacy.md#how-to-determine-visibility-of-a-type>.
/// The default type visitor (`TypeVisitor`) does most of the job, but it has some shortcomings.
/// First, it doesn't have overridable `fn visit_trait_ref`, so we have to catch trait `DefId`s
/// manually. Second, it doesn't visit some type components like signatures of fn types, or traits
/// in `impl Trait`, see individual comments in `DefIdVisitorSkeleton::visit_ty`.
trait DefIdVisitor<'tcx> {
    type BreakTy = ();

    fn tcx(&self) -> TyCtxt<'tcx>;
    fn shallow(&self) -> bool {
        false
    }
    fn skip_assoc_tys(&self) -> bool {
        false
    }
    fn visit_def_id(
        &mut self,
        def_id: DefId,
        kind: &str,
        descr: &dyn fmt::Display,
    ) -> ControlFlow<Self::BreakTy>;

    /// Not overridden, but used to actually visit types and traits.
    fn skeleton(&mut self) -> DefIdVisitorSkeleton<'_, 'tcx, Self> {
        DefIdVisitorSkeleton {
            def_id_visitor: self,
            visited_opaque_tys: Default::default(),
            dummy: Default::default(),
        }
    }
    fn visit(&mut self, ty_fragment: impl TypeFoldable<'tcx>) -> ControlFlow<Self::BreakTy> {
        ty_fragment.visit_with(&mut self.skeleton())
    }
    fn visit_trait(&mut self, trait_ref: TraitRef<'tcx>) -> ControlFlow<Self::BreakTy> {
        self.skeleton().visit_trait(trait_ref)
    }
    fn visit_projection_ty(
        &mut self,
        projection: ty::ProjectionTy<'tcx>,
    ) -> ControlFlow<Self::BreakTy> {
        self.skeleton().visit_projection_ty(projection)
    }
    fn visit_predicates(
        &mut self,
        predicates: ty::GenericPredicates<'tcx>,
    ) -> ControlFlow<Self::BreakTy> {
        self.skeleton().visit_predicates(predicates)
    }
}

struct DefIdVisitorSkeleton<'v, 'tcx, V: ?Sized> {
    def_id_visitor: &'v mut V,
    visited_opaque_tys: FxHashSet<DefId>,
    dummy: PhantomData<TyCtxt<'tcx>>,
}

impl<'tcx, V> DefIdVisitorSkeleton<'_, 'tcx, V>
where
    V: DefIdVisitor<'tcx> + ?Sized,
{
    fn visit_trait(&mut self, trait_ref: TraitRef<'tcx>) -> ControlFlow<V::BreakTy> {
        let TraitRef { def_id, substs } = trait_ref;
        self.def_id_visitor.visit_def_id(def_id, "trait", &trait_ref.print_only_trait_path())?;
        if self.def_id_visitor.shallow() { ControlFlow::CONTINUE } else { substs.visit_with(self) }
    }

    fn visit_projection_ty(
        &mut self,
        projection: ty::ProjectionTy<'tcx>,
    ) -> ControlFlow<V::BreakTy> {
        let (trait_ref, assoc_substs) =
            projection.trait_ref_and_own_substs(self.def_id_visitor.tcx());
        self.visit_trait(trait_ref)?;
        if self.def_id_visitor.shallow() {
            ControlFlow::CONTINUE
        } else {
            assoc_substs.iter().try_for_each(|subst| subst.visit_with(self))
        }
    }

    fn visit_predicate(&mut self, predicate: ty::Predicate<'tcx>) -> ControlFlow<V::BreakTy> {
        match predicate.kind().skip_binder() {
            ty::PredicateKind::Trait(ty::TraitPredicate {
                trait_ref,
                constness: _,
                polarity: _,
            }) => self.visit_trait(trait_ref),
            ty::PredicateKind::Projection(ty::ProjectionPredicate { projection_ty, ty }) => {
                ty.visit_with(self)?;
                self.visit_projection_ty(projection_ty)
            }
            ty::PredicateKind::TypeOutlives(ty::OutlivesPredicate(ty, _region)) => {
                ty.visit_with(self)
            }
            ty::PredicateKind::RegionOutlives(..) => ControlFlow::CONTINUE,
            ty::PredicateKind::ConstEvaluatable(uv)
                if self.def_id_visitor.tcx().features().generic_const_exprs =>
            {
                let tcx = self.def_id_visitor.tcx();
                if let Ok(Some(ct)) = AbstractConst::new(tcx, uv) {
                    self.visit_abstract_const_expr(tcx, ct)?;
                }
                ControlFlow::CONTINUE
            }
            _ => bug!("unexpected predicate: {:?}", predicate),
        }
    }

    fn visit_abstract_const_expr(
        &mut self,
        tcx: TyCtxt<'tcx>,
        ct: AbstractConst<'tcx>,
    ) -> ControlFlow<V::BreakTy> {
        const_evaluatable::walk_abstract_const(tcx, ct, |node| match node.root(tcx) {
            ACNode::Leaf(leaf) => self.visit_const(leaf),
            ACNode::Cast(_, _, ty) => self.visit_ty(ty),
            ACNode::Binop(..) | ACNode::UnaryOp(..) | ACNode::FunctionCall(_, _) => {
                ControlFlow::CONTINUE
            }
        })
    }

    fn visit_predicates(
        &mut self,
        predicates: ty::GenericPredicates<'tcx>,
    ) -> ControlFlow<V::BreakTy> {
        let ty::GenericPredicates { parent: _, predicates } = predicates;
        predicates.iter().try_for_each(|&(predicate, _span)| self.visit_predicate(predicate))
    }
}

impl<'tcx, V> TypeVisitor<'tcx> for DefIdVisitorSkeleton<'_, 'tcx, V>
where
    V: DefIdVisitor<'tcx> + ?Sized,
{
    type BreakTy = V::BreakTy;

    fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
        Some(self.def_id_visitor.tcx())
    }

    fn visit_ty(&mut self, ty: Ty<'tcx>) -> ControlFlow<V::BreakTy> {
        let tcx = self.def_id_visitor.tcx();
        // InternalSubsts are not visited here because they are visited below in `super_visit_with`.
        match *ty.kind() {
            ty::Adt(&ty::AdtDef { did: def_id, .. }, ..)
            | ty::Foreign(def_id)
            | ty::FnDef(def_id, ..)
            | ty::Closure(def_id, ..)
            | ty::Generator(def_id, ..) => {
                self.def_id_visitor.visit_def_id(def_id, "type", &ty)?;
                if self.def_id_visitor.shallow() {
                    return ControlFlow::CONTINUE;
                }
                // Default type visitor doesn't visit signatures of fn types.
                // Something like `fn() -> Priv {my_func}` is considered a private type even if
                // `my_func` is public, so we need to visit signatures.
                if let ty::FnDef(..) = ty.kind() {
                    tcx.fn_sig(def_id).visit_with(self)?;
                }
                // Inherent static methods don't have self type in substs.
                // Something like `fn() {my_method}` type of the method
                // `impl Pub<Priv> { pub fn my_method() {} }` is considered a private type,
                // so we need to visit the self type additionally.
                if let Some(assoc_item) = tcx.opt_associated_item(def_id) {
                    if let ty::ImplContainer(impl_def_id) = assoc_item.container {
                        tcx.type_of(impl_def_id).visit_with(self)?;
                    }
                }
            }
            ty::Projection(proj) => {
                if self.def_id_visitor.skip_assoc_tys() {
                    // Visitors searching for minimal visibility/reachability want to
                    // conservatively approximate associated types like `<Type as Trait>::Alias`
                    // as visible/reachable even if both `Type` and `Trait` are private.
                    // Ideally, associated types should be substituted in the same way as
                    // free type aliases, but this isn't done yet.
                    return ControlFlow::CONTINUE;
                }
                // This will also visit substs if necessary, so we don't need to recurse.
                return self.visit_projection_ty(proj);
            }
            ty::Dynamic(predicates, ..) => {
                // All traits in the list are considered the "primary" part of the type
                // and are visited by shallow visitors.
                for predicate in predicates {
                    let trait_ref = match predicate.skip_binder() {
                        ty::ExistentialPredicate::Trait(trait_ref) => trait_ref,
                        ty::ExistentialPredicate::Projection(proj) => proj.trait_ref(tcx),
                        ty::ExistentialPredicate::AutoTrait(def_id) => {
                            ty::ExistentialTraitRef { def_id, substs: InternalSubsts::empty() }
                        }
                    };
                    let ty::ExistentialTraitRef { def_id, substs: _ } = trait_ref;
                    self.def_id_visitor.visit_def_id(def_id, "trait", &trait_ref)?;
                }
            }
            ty::Opaque(def_id, ..) => {
                // Skip repeated `Opaque`s to avoid infinite recursion.
                if self.visited_opaque_tys.insert(def_id) {
                    // The intent is to treat `impl Trait1 + Trait2` identically to
                    // `dyn Trait1 + Trait2`. Therefore we ignore def-id of the opaque type itself
                    // (it either has no visibility, or its visibility is insignificant, like
                    // visibilities of type aliases) and recurse into bounds instead to go
                    // through the trait list (default type visitor doesn't visit those traits).
                    // All traits in the list are considered the "primary" part of the type
                    // and are visited by shallow visitors.
                    self.visit_predicates(ty::GenericPredicates {
                        parent: None,
                        predicates: tcx.explicit_item_bounds(def_id),
                    })?;
                }
            }
            // These types don't have their own def-ids (but may have subcomponents
            // with def-ids that should be visited recursively).
            ty::Bool
            | ty::Char
            | ty::Int(..)
            | ty::Uint(..)
            | ty::Float(..)
            | ty::Str
            | ty::Never
            | ty::Array(..)
            | ty::Slice(..)
            | ty::Tuple(..)
            | ty::RawPtr(..)
            | ty::Ref(..)
            | ty::FnPtr(..)
            | ty::Param(..)
            | ty::Error(_)
            | ty::GeneratorWitness(..) => {}
            ty::Bound(..) | ty::Placeholder(..) | ty::Infer(..) => {
                bug!("unexpected type: {:?}", ty)
            }
        }

        if self.def_id_visitor.shallow() {
            ControlFlow::CONTINUE
        } else {
            ty.super_visit_with(self)
        }
    }

    fn visit_const(&mut self, c: &'tcx Const<'tcx>) -> ControlFlow<Self::BreakTy> {
        self.visit_ty(c.ty)?;
        let tcx = self.def_id_visitor.tcx();
        if let Ok(Some(ct)) = AbstractConst::from_const(tcx, c) {
            self.visit_abstract_const_expr(tcx, ct)?;
        }
        ControlFlow::CONTINUE
    }
}

fn min(vis1: ty::Visibility, vis2: ty::Visibility, tcx: TyCtxt<'_>) -> ty::Visibility {
    if vis1.is_at_least(vis2, tcx) { vis2 } else { vis1 }
}

////////////////////////////////////////////////////////////////////////////////
/// Visitor used to determine if pub(restricted) is used anywhere in the crate.
///
/// This is done so that `private_in_public` warnings can be turned into hard errors
/// in crates that have been updated to use pub(restricted).
////////////////////////////////////////////////////////////////////////////////
struct PubRestrictedVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    has_pub_restricted: bool,
}

impl Visitor<'tcx> for PubRestrictedVisitor<'tcx> {
    type Map = Map<'tcx>;

    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::All(self.tcx.hir())
    }
    fn visit_vis(&mut self, vis: &'tcx hir::Visibility<'tcx>) {
        self.has_pub_restricted = self.has_pub_restricted || vis.node.is_pub_restricted();
    }
}

////////////////////////////////////////////////////////////////////////////////
/// Visitor used to determine impl visibility and reachability.
////////////////////////////////////////////////////////////////////////////////

struct FindMin<'a, 'tcx, VL: VisibilityLike> {
    tcx: TyCtxt<'tcx>,
    access_levels: &'a AccessLevels,
    min: VL,
}

impl<'a, 'tcx, VL: VisibilityLike> DefIdVisitor<'tcx> for FindMin<'a, 'tcx, VL> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }
    fn shallow(&self) -> bool {
        VL::SHALLOW
    }
    fn skip_assoc_tys(&self) -> bool {
        true
    }
    fn visit_def_id(
        &mut self,
        def_id: DefId,
        _kind: &str,
        _descr: &dyn fmt::Display,
    ) -> ControlFlow<Self::BreakTy> {
        self.min = VL::new_min(self, def_id);
        ControlFlow::CONTINUE
    }
}

trait VisibilityLike: Sized {
    const MAX: Self;
    const SHALLOW: bool = false;
    fn new_min(find: &FindMin<'_, '_, Self>, def_id: DefId) -> Self;

    // Returns an over-approximation (`skip_assoc_tys` = true) of visibility due to
    // associated types for which we can't determine visibility precisely.
    fn of_impl(def_id: LocalDefId, tcx: TyCtxt<'_>, access_levels: &AccessLevels) -> Self {
        let mut find = FindMin { tcx, access_levels, min: Self::MAX };
        find.visit(tcx.type_of(def_id));
        if let Some(trait_ref) = tcx.impl_trait_ref(def_id) {
            find.visit_trait(trait_ref);
        }
        find.min
    }
}
impl VisibilityLike for ty::Visibility {
    const MAX: Self = ty::Visibility::Public;
    fn new_min(find: &FindMin<'_, '_, Self>, def_id: DefId) -> Self {
        min(find.tcx.visibility(def_id), find.min, find.tcx)
    }
}
impl VisibilityLike for Option<AccessLevel> {
    const MAX: Self = Some(AccessLevel::Public);
    // Type inference is very smart sometimes.
    // It can make an impl reachable even some components of its type or trait are unreachable.
    // E.g. methods of `impl ReachableTrait<UnreachableTy> for ReachableTy<UnreachableTy> { ... }`
    // can be usable from other crates (#57264). So we skip substs when calculating reachability
    // and consider an impl reachable if its "shallow" type and trait are reachable.
    //
    // The assumption we make here is that type-inference won't let you use an impl without knowing
    // both "shallow" version of its self type and "shallow" version of its trait if it exists
    // (which require reaching the `DefId`s in them).
    const SHALLOW: bool = true;
    fn new_min(find: &FindMin<'_, '_, Self>, def_id: DefId) -> Self {
        cmp::min(
            if let Some(def_id) = def_id.as_local() {
                find.access_levels.map.get(&def_id).copied()
            } else {
                Self::MAX
            },
            find.min,
        )
    }
}

////////////////////////////////////////////////////////////////////////////////
/// The embargo visitor, used to determine the exports of the AST.
////////////////////////////////////////////////////////////////////////////////

struct EmbargoVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,

    /// Accessibility levels for reachable nodes.
    access_levels: AccessLevels,
    /// A set of pairs corresponding to modules, where the first module is
    /// reachable via a macro that's defined in the second module. This cannot
    /// be represented as reachable because it can't handle the following case:
    ///
    /// pub mod n {                         // Should be `Public`
    ///     pub(crate) mod p {              // Should *not* be accessible
    ///         pub fn f() -> i32 { 12 }    // Must be `Reachable`
    ///     }
    /// }
    /// pub macro m() {
    ///     n::p::f()
    /// }
    macro_reachable: FxHashSet<(LocalDefId, LocalDefId)>,
    /// Previous accessibility level; `None` means unreachable.
    prev_level: Option<AccessLevel>,
    /// Has something changed in the level map?
    changed: bool,
}

struct ReachEverythingInTheInterfaceVisitor<'a, 'tcx> {
    access_level: Option<AccessLevel>,
    item_def_id: LocalDefId,
    ev: &'a mut EmbargoVisitor<'tcx>,
}

impl EmbargoVisitor<'tcx> {
    fn get(&self, def_id: LocalDefId) -> Option<AccessLevel> {
        self.access_levels.map.get(&def_id).copied()
    }

    /// Updates node level and returns the updated level.
    fn update(&mut self, def_id: LocalDefId, level: Option<AccessLevel>) -> Option<AccessLevel> {
        let old_level = self.get(def_id);
        // Accessibility levels can only grow.
        if level > old_level {
            self.access_levels.map.insert(def_id, level.unwrap());
            self.changed = true;
            level
        } else {
            old_level
        }
    }

    fn reach(
        &mut self,
        def_id: LocalDefId,
        access_level: Option<AccessLevel>,
    ) -> ReachEverythingInTheInterfaceVisitor<'_, 'tcx> {
        ReachEverythingInTheInterfaceVisitor {
            access_level: cmp::min(access_level, Some(AccessLevel::Reachable)),
            item_def_id: def_id,
            ev: self,
        }
    }

    // We have to make sure that the items that macros might reference
    // are reachable, since they might be exported transitively.
    fn update_reachability_from_macro(&mut self, local_def_id: LocalDefId, md: &MacroDef) {
        // Non-opaque macros cannot make other items more accessible than they already are.

        let hir_id = self.tcx.hir().local_def_id_to_hir_id(local_def_id);
        let attrs = self.tcx.hir().attrs(hir_id);
        if attr::find_transparency(attrs, md.macro_rules).0 != Transparency::Opaque {
            return;
        }

        let item_def_id = local_def_id.to_def_id();
        let macro_module_def_id =
            ty::DefIdTree::parent(self.tcx, item_def_id).unwrap().expect_local();
        if self.tcx.hir().opt_def_kind(macro_module_def_id) != Some(DefKind::Mod) {
            // The macro's parent doesn't correspond to a `mod`, return early (#63164, #65252).
            return;
        }

        if self.get(local_def_id).is_none() {
            return;
        }

        // Since we are starting from an externally visible module,
        // all the parents in the loop below are also guaranteed to be modules.
        let mut module_def_id = macro_module_def_id;
        loop {
            let changed_reachability =
                self.update_macro_reachable(module_def_id, macro_module_def_id);
            if changed_reachability || module_def_id == CRATE_DEF_ID {
                break;
            }
            module_def_id =
                ty::DefIdTree::parent(self.tcx, module_def_id.to_def_id()).unwrap().expect_local();
        }
    }

    /// Updates the item as being reachable through a macro defined in the given
    /// module. Returns `true` if the level has changed.
    fn update_macro_reachable(
        &mut self,
        module_def_id: LocalDefId,
        defining_mod: LocalDefId,
    ) -> bool {
        if self.macro_reachable.insert((module_def_id, defining_mod)) {
            self.update_macro_reachable_mod(module_def_id, defining_mod);
            true
        } else {
            false
        }
    }

    fn update_macro_reachable_mod(&mut self, module_def_id: LocalDefId, defining_mod: LocalDefId) {
        let module = self.tcx.hir().get_module(module_def_id).0;
        for item_id in module.item_ids {
            let def_kind = self.tcx.def_kind(item_id.def_id);
            let vis = self.tcx.visibility(item_id.def_id);
            self.update_macro_reachable_def(item_id.def_id, def_kind, vis, defining_mod);
        }
        if let Some(exports) = self.tcx.module_exports(module_def_id) {
            for export in exports {
                if export.vis.is_accessible_from(defining_mod.to_def_id(), self.tcx) {
                    if let Res::Def(def_kind, def_id) = export.res {
                        if let Some(def_id) = def_id.as_local() {
                            let vis = self.tcx.visibility(def_id.to_def_id());
                            self.update_macro_reachable_def(def_id, def_kind, vis, defining_mod);
                        }
                    }
                }
            }
        }
    }

    fn update_macro_reachable_def(
        &mut self,
        def_id: LocalDefId,
        def_kind: DefKind,
        vis: ty::Visibility,
        module: LocalDefId,
    ) {
        let level = Some(AccessLevel::Reachable);
        if let ty::Visibility::Public = vis {
            self.update(def_id, level);
        }
        match def_kind {
            // No type privacy, so can be directly marked as reachable.
            DefKind::Const | DefKind::Static | DefKind::TraitAlias | DefKind::TyAlias => {
                if vis.is_accessible_from(module.to_def_id(), self.tcx) {
                    self.update(def_id, level);
                }
            }

            // Hygine isn't really implemented for `macro_rules!` macros at the
            // moment. Accordingly, marking them as reachable is unwise. `macro` macros
            // have normal  hygine, so we can treat them like other items without type
            // privacy and mark them reachable.
            DefKind::Macro(_) => {
                let hir_id = self.tcx.hir().local_def_id_to_hir_id(def_id);
                let item = self.tcx.hir().expect_item(hir_id);
                if let hir::ItemKind::Macro(MacroDef { macro_rules: false, .. }) = item.kind {
                    if vis.is_accessible_from(module.to_def_id(), self.tcx) {
                        self.update(def_id, level);
                    }
                }
            }

            // We can't use a module name as the final segment of a path, except
            // in use statements. Since re-export checking doesn't consider
            // hygiene these don't need to be marked reachable. The contents of
            // the module, however may be reachable.
            DefKind::Mod => {
                if vis.is_accessible_from(module.to_def_id(), self.tcx) {
                    self.update_macro_reachable(def_id, module);
                }
            }

            DefKind::Struct | DefKind::Union => {
                // While structs and unions have type privacy, their fields do not.
                if let ty::Visibility::Public = vis {
                    let item =
                        self.tcx.hir().expect_item(self.tcx.hir().local_def_id_to_hir_id(def_id));
                    if let hir::ItemKind::Struct(ref struct_def, _)
                    | hir::ItemKind::Union(ref struct_def, _) = item.kind
                    {
                        for field in struct_def.fields() {
                            let def_id = self.tcx.hir().local_def_id(field.hir_id);
                            let field_vis = self.tcx.visibility(def_id);
                            if field_vis.is_accessible_from(module.to_def_id(), self.tcx) {
                                self.reach(def_id, level).ty();
                            }
                        }
                    } else {
                        bug!("item {:?} with DefKind {:?}", item, def_kind);
                    }
                }
            }

            // These have type privacy, so are not reachable unless they're
            // public, or are not namespaced at all.
            DefKind::AssocConst
            | DefKind::AssocTy
            | DefKind::ConstParam
            | DefKind::Ctor(_, _)
            | DefKind::Enum
            | DefKind::ForeignTy
            | DefKind::Fn
            | DefKind::OpaqueTy
            | DefKind::AssocFn
            | DefKind::Trait
            | DefKind::TyParam
            | DefKind::Variant
            | DefKind::LifetimeParam
            | DefKind::ExternCrate
            | DefKind::Use
            | DefKind::ForeignMod
            | DefKind::AnonConst
            | DefKind::Field
            | DefKind::GlobalAsm
            | DefKind::Impl
            | DefKind::Closure
            | DefKind::Generator => (),
        }
    }

    /// Given the path segments of an `ItemKind::Use`, then we need
    /// to update the visibility of the intermediate use so that it isn't linted
    /// by `unreachable_pub`.
    ///
    /// This isn't trivial as `path.res` has the `DefId` of the eventual target
    /// of the use statement not of the next intermediate use statement.
    ///
    /// To do this, consider the last two segments of the path to our intermediate
    /// use statement. We expect the penultimate segment to be a module and the
    /// last segment to be the name of the item we are exporting. We can then
    /// look at the items contained in the module for the use statement with that
    /// name and update that item's visibility.
    ///
    /// FIXME: This solution won't work with glob imports and doesn't respect
    /// namespaces. See <https://github.com/rust-lang/rust/pull/57922#discussion_r251234202>.
    fn update_visibility_of_intermediate_use_statements(
        &mut self,
        segments: &[hir::PathSegment<'_>],
    ) {
        if let [.., module, segment] = segments {
            if let Some(item) = module
                .res
                .and_then(|res| res.mod_def_id())
                // If the module is `self`, i.e. the current crate,
                // there will be no corresponding item.
                .filter(|def_id| def_id.index != CRATE_DEF_INDEX || def_id.krate != LOCAL_CRATE)
                .and_then(|def_id| {
                    def_id.as_local().map(|def_id| self.tcx.hir().local_def_id_to_hir_id(def_id))
                })
                .map(|module_hir_id| self.tcx.hir().expect_item(module_hir_id))
            {
                if let hir::ItemKind::Mod(m) = &item.kind {
                    for &item_id in m.item_ids {
                        let item = self.tcx.hir().item(item_id);
                        if !self.tcx.hygienic_eq(
                            segment.ident,
                            item.ident,
                            item_id.def_id.to_def_id(),
                        ) {
                            continue;
                        }
                        if let hir::ItemKind::Use(..) = item.kind {
                            self.update(item.def_id, Some(AccessLevel::Exported));
                        }
                    }
                }
            }
        }
    }
}

impl Visitor<'tcx> for EmbargoVisitor<'tcx> {
    type Map = Map<'tcx>;

    /// We want to visit items in the context of their containing
    /// module and so forth, so supply a crate for doing a deep walk.
    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::All(self.tcx.hir())
    }

    fn visit_item(&mut self, item: &'tcx hir::Item<'tcx>) {
        let inherited_item_level = match item.kind {
            hir::ItemKind::Impl { .. } => {
                Option::<AccessLevel>::of_impl(item.def_id, self.tcx, &self.access_levels)
            }
            // Only exported `macro_rules!` items are public, but they always are.
            hir::ItemKind::Macro(MacroDef { macro_rules: true, .. }) => {
                let def_id = item.def_id.to_def_id();
                let is_macro_export = self.tcx.has_attr(def_id, sym::macro_export);
                if is_macro_export { Some(AccessLevel::Public) } else { None }
            }
            // Foreign modules inherit level from parents.
            hir::ItemKind::ForeignMod { .. } => self.prev_level,
            // Other `pub` items inherit levels from parents.
            hir::ItemKind::Const(..)
            | hir::ItemKind::Enum(..)
            | hir::ItemKind::ExternCrate(..)
            | hir::ItemKind::GlobalAsm(..)
            | hir::ItemKind::Fn(..)
            | hir::ItemKind::Macro(..)
            | hir::ItemKind::Mod(..)
            | hir::ItemKind::Static(..)
            | hir::ItemKind::Struct(..)
            | hir::ItemKind::Trait(..)
            | hir::ItemKind::TraitAlias(..)
            | hir::ItemKind::OpaqueTy(..)
            | hir::ItemKind::TyAlias(..)
            | hir::ItemKind::Union(..)
            | hir::ItemKind::Use(..) => {
                if item.vis.node.is_pub() {
                    self.prev_level
                } else {
                    None
                }
            }
        };

        // Update level of the item itself.
        let item_level = self.update(item.def_id, inherited_item_level);

        // Update levels of nested things.
        match item.kind {
            hir::ItemKind::Enum(ref def, _) => {
                for variant in def.variants {
                    let variant_level =
                        self.update(self.tcx.hir().local_def_id(variant.id), item_level);
                    if let Some(ctor_hir_id) = variant.data.ctor_hir_id() {
                        self.update(self.tcx.hir().local_def_id(ctor_hir_id), item_level);
                    }
                    for field in variant.data.fields() {
                        self.update(self.tcx.hir().local_def_id(field.hir_id), variant_level);
                    }
                }
            }
            hir::ItemKind::Impl(ref impl_) => {
                for impl_item_ref in impl_.items {
                    if impl_.of_trait.is_some()
                        || self.tcx.visibility(impl_item_ref.id.def_id) == ty::Visibility::Public
                    {
                        self.update(impl_item_ref.id.def_id, item_level);
                    }
                }
            }
            hir::ItemKind::Trait(.., trait_item_refs) => {
                for trait_item_ref in trait_item_refs {
                    self.update(trait_item_ref.id.def_id, item_level);
                }
            }
            hir::ItemKind::Struct(ref def, _) | hir::ItemKind::Union(ref def, _) => {
                if let Some(ctor_hir_id) = def.ctor_hir_id() {
                    self.update(self.tcx.hir().local_def_id(ctor_hir_id), item_level);
                }
                for field in def.fields() {
                    if field.vis.node.is_pub() {
                        self.update(self.tcx.hir().local_def_id(field.hir_id), item_level);
                    }
                }
            }
            hir::ItemKind::Macro(ref macro_def) => {
                self.update_reachability_from_macro(item.def_id, macro_def);
            }
            hir::ItemKind::ForeignMod { items, .. } => {
                for foreign_item in items {
                    if self.tcx.visibility(foreign_item.id.def_id) == ty::Visibility::Public {
                        self.update(foreign_item.id.def_id, item_level);
                    }
                }
            }

            hir::ItemKind::OpaqueTy(..)
            | hir::ItemKind::Use(..)
            | hir::ItemKind::Static(..)
            | hir::ItemKind::Const(..)
            | hir::ItemKind::GlobalAsm(..)
            | hir::ItemKind::TyAlias(..)
            | hir::ItemKind::Mod(..)
            | hir::ItemKind::TraitAlias(..)
            | hir::ItemKind::Fn(..)
            | hir::ItemKind::ExternCrate(..) => {}
        }

        // Mark all items in interfaces of reachable items as reachable.
        match item.kind {
            // The interface is empty.
            hir::ItemKind::Macro(..) | hir::ItemKind::ExternCrate(..) => {}
            // All nested items are checked by `visit_item`.
            hir::ItemKind::Mod(..) => {}
            // Re-exports are handled in `visit_mod`. However, in order to avoid looping over
            // all of the items of a mod in `visit_mod` looking for use statements, we handle
            // making sure that intermediate use statements have their visibilities updated here.
            hir::ItemKind::Use(path, _) => {
                if item_level.is_some() {
                    self.update_visibility_of_intermediate_use_statements(path.segments.as_ref());
                }
            }
            // The interface is empty.
            hir::ItemKind::GlobalAsm(..) => {}
            hir::ItemKind::OpaqueTy(..) => {
                // HACK(jynelson): trying to infer the type of `impl trait` breaks `async-std` (and `pub async fn` in general)
                // Since rustdoc never needs to do codegen and doesn't care about link-time reachability,
                // mark this as unreachable.
                // See https://github.com/rust-lang/rust/issues/75100
                if !self.tcx.sess.opts.actually_rustdoc {
                    // FIXME: This is some serious pessimization intended to workaround deficiencies
                    // in the reachability pass (`middle/reachable.rs`). Types are marked as link-time
                    // reachable if they are returned via `impl Trait`, even from private functions.
                    let exist_level =
                        cmp::max(item_level, Some(AccessLevel::ReachableFromImplTrait));
                    self.reach(item.def_id, exist_level).generics().predicates().ty();
                }
            }
            // Visit everything.
            hir::ItemKind::Const(..)
            | hir::ItemKind::Static(..)
            | hir::ItemKind::Fn(..)
            | hir::ItemKind::TyAlias(..) => {
                if item_level.is_some() {
                    self.reach(item.def_id, item_level).generics().predicates().ty();
                }
            }
            hir::ItemKind::Trait(.., trait_item_refs) => {
                if item_level.is_some() {
                    self.reach(item.def_id, item_level).generics().predicates();

                    for trait_item_ref in trait_item_refs {
                        let mut reach = self.reach(trait_item_ref.id.def_id, item_level);
                        reach.generics().predicates();

                        if trait_item_ref.kind == AssocItemKind::Type
                            && !trait_item_ref.defaultness.has_value()
                        {
                            // No type to visit.
                        } else {
                            reach.ty();
                        }
                    }
                }
            }
            hir::ItemKind::TraitAlias(..) => {
                if item_level.is_some() {
                    self.reach(item.def_id, item_level).generics().predicates();
                }
            }
            // Visit everything except for private impl items.
            hir::ItemKind::Impl(ref impl_) => {
                if item_level.is_some() {
                    self.reach(item.def_id, item_level).generics().predicates().ty().trait_ref();

                    for impl_item_ref in impl_.items {
                        let impl_item_level = self.get(impl_item_ref.id.def_id);
                        if impl_item_level.is_some() {
                            self.reach(impl_item_ref.id.def_id, impl_item_level)
                                .generics()
                                .predicates()
                                .ty();
                        }
                    }
                }
            }

            // Visit everything, but enum variants have their own levels.
            hir::ItemKind::Enum(ref def, _) => {
                if item_level.is_some() {
                    self.reach(item.def_id, item_level).generics().predicates();
                }
                for variant in def.variants {
                    let variant_level = self.get(self.tcx.hir().local_def_id(variant.id));
                    if variant_level.is_some() {
                        for field in variant.data.fields() {
                            self.reach(self.tcx.hir().local_def_id(field.hir_id), variant_level)
                                .ty();
                        }
                        // Corner case: if the variant is reachable, but its
                        // enum is not, make the enum reachable as well.
                        self.update(item.def_id, variant_level);
                    }
                }
            }
            // Visit everything, but foreign items have their own levels.
            hir::ItemKind::ForeignMod { items, .. } => {
                for foreign_item in items {
                    let foreign_item_level = self.get(foreign_item.id.def_id);
                    if foreign_item_level.is_some() {
                        self.reach(foreign_item.id.def_id, foreign_item_level)
                            .generics()
                            .predicates()
                            .ty();
                    }
                }
            }
            // Visit everything except for private fields.
            hir::ItemKind::Struct(ref struct_def, _) | hir::ItemKind::Union(ref struct_def, _) => {
                if item_level.is_some() {
                    self.reach(item.def_id, item_level).generics().predicates();
                    for field in struct_def.fields() {
                        let def_id = self.tcx.hir().local_def_id(field.hir_id);
                        let field_level = self.get(def_id);
                        if field_level.is_some() {
                            self.reach(def_id, field_level).ty();
                        }
                    }
                }
            }
        }

        let orig_level = mem::replace(&mut self.prev_level, item_level);
        intravisit::walk_item(self, item);
        self.prev_level = orig_level;
    }

    fn visit_block(&mut self, b: &'tcx hir::Block<'tcx>) {
        // Blocks can have public items, for example impls, but they always
        // start as completely private regardless of publicity of a function,
        // constant, type, field, etc., in which this block resides.
        let orig_level = mem::replace(&mut self.prev_level, None);
        intravisit::walk_block(self, b);
        self.prev_level = orig_level;
    }

    fn visit_mod(&mut self, m: &'tcx hir::Mod<'tcx>, _sp: Span, id: hir::HirId) {
        // This code is here instead of in visit_item so that the
        // crate module gets processed as well.
        if self.prev_level.is_some() {
            let def_id = self.tcx.hir().local_def_id(id);
            if let Some(exports) = self.tcx.module_exports(def_id) {
                for export in exports.iter() {
                    if export.vis == ty::Visibility::Public {
                        if let Some(def_id) = export.res.opt_def_id() {
                            if let Some(def_id) = def_id.as_local() {
                                self.update(def_id, Some(AccessLevel::Exported));
                            }
                        }
                    }
                }
            }
        }

        intravisit::walk_mod(self, m, id);
    }
}

impl ReachEverythingInTheInterfaceVisitor<'_, 'tcx> {
    fn generics(&mut self) -> &mut Self {
        for param in &self.ev.tcx.generics_of(self.item_def_id).params {
            match param.kind {
                GenericParamDefKind::Lifetime => {}
                GenericParamDefKind::Type { has_default, .. } => {
                    if has_default {
                        self.visit(self.ev.tcx.type_of(param.def_id));
                    }
                }
                GenericParamDefKind::Const { has_default, .. } => {
                    self.visit(self.ev.tcx.type_of(param.def_id));
                    if has_default {
                        self.visit(self.ev.tcx.const_param_default(param.def_id));
                    }
                }
            }
        }
        self
    }

    fn predicates(&mut self) -> &mut Self {
        self.visit_predicates(self.ev.tcx.predicates_of(self.item_def_id));
        self
    }

    fn ty(&mut self) -> &mut Self {
        self.visit(self.ev.tcx.type_of(self.item_def_id));
        self
    }

    fn trait_ref(&mut self) -> &mut Self {
        if let Some(trait_ref) = self.ev.tcx.impl_trait_ref(self.item_def_id) {
            self.visit_trait(trait_ref);
        }
        self
    }
}

impl DefIdVisitor<'tcx> for ReachEverythingInTheInterfaceVisitor<'_, 'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.ev.tcx
    }
    fn visit_def_id(
        &mut self,
        def_id: DefId,
        _kind: &str,
        _descr: &dyn fmt::Display,
    ) -> ControlFlow<Self::BreakTy> {
        if let Some(def_id) = def_id.as_local() {
            if let (ty::Visibility::Public, _) | (_, Some(AccessLevel::ReachableFromImplTrait)) =
                (self.tcx().visibility(def_id.to_def_id()), self.access_level)
            {
                self.ev.update(def_id, self.access_level);
            }
        }
        ControlFlow::CONTINUE
    }
}

//////////////////////////////////////////////////////////////////////////////////////
/// Name privacy visitor, checks privacy and reports violations.
/// Most of name privacy checks are performed during the main resolution phase,
/// or later in type checking when field accesses and associated items are resolved.
/// This pass performs remaining checks for fields in struct expressions and patterns.
//////////////////////////////////////////////////////////////////////////////////////

struct NamePrivacyVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    maybe_typeck_results: Option<&'tcx ty::TypeckResults<'tcx>>,
    current_item: LocalDefId,
}

impl<'tcx> NamePrivacyVisitor<'tcx> {
    /// Gets the type-checking results for the current body.
    /// As this will ICE if called outside bodies, only call when working with
    /// `Expr` or `Pat` nodes (they are guaranteed to be found only in bodies).
    #[track_caller]
    fn typeck_results(&self) -> &'tcx ty::TypeckResults<'tcx> {
        self.maybe_typeck_results
            .expect("`NamePrivacyVisitor::typeck_results` called outside of body")
    }

    // Checks that a field in a struct constructor (expression or pattern) is accessible.
    fn check_field(
        &mut self,
        use_ctxt: Span,        // syntax context of the field name at the use site
        span: Span,            // span of the field pattern, e.g., `x: 0`
        def: &'tcx ty::AdtDef, // definition of the struct or enum
        field: &'tcx ty::FieldDef,
        in_update_syntax: bool,
    ) {
        if def.is_enum() {
            return;
        }

        // definition of the field
        let ident = Ident::new(kw::Empty, use_ctxt);
        let hir_id = self.tcx.hir().local_def_id_to_hir_id(self.current_item);
        let def_id = self.tcx.adjust_ident_and_get_scope(ident, def.did, hir_id).1;
        if !field.vis.is_accessible_from(def_id, self.tcx) {
            let label = if in_update_syntax {
                format!("field `{}` is private", field.ident)
            } else {
                "private field".to_string()
            };

            struct_span_err!(
                self.tcx.sess,
                span,
                E0451,
                "field `{}` of {} `{}` is private",
                field.ident,
                def.variant_descr(),
                self.tcx.def_path_str(def.did)
            )
            .span_label(span, label)
            .emit();
        }
    }
}

impl<'tcx> Visitor<'tcx> for NamePrivacyVisitor<'tcx> {
    type Map = Map<'tcx>;

    /// We want to visit items in the context of their containing
    /// module and so forth, so supply a crate for doing a deep walk.
    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::All(self.tcx.hir())
    }

    fn visit_mod(&mut self, _m: &'tcx hir::Mod<'tcx>, _s: Span, _n: hir::HirId) {
        // Don't visit nested modules, since we run a separate visitor walk
        // for each module in `privacy_access_levels`
    }

    fn visit_nested_body(&mut self, body: hir::BodyId) {
        let old_maybe_typeck_results =
            self.maybe_typeck_results.replace(self.tcx.typeck_body(body));
        let body = self.tcx.hir().body(body);
        self.visit_body(body);
        self.maybe_typeck_results = old_maybe_typeck_results;
    }

    fn visit_item(&mut self, item: &'tcx hir::Item<'tcx>) {
        let orig_current_item = mem::replace(&mut self.current_item, item.def_id);
        intravisit::walk_item(self, item);
        self.current_item = orig_current_item;
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::Struct(qpath, fields, ref base) = expr.kind {
            let res = self.typeck_results().qpath_res(qpath, expr.hir_id);
            let adt = self.typeck_results().expr_ty(expr).ty_adt_def().unwrap();
            let variant = adt.variant_of_res(res);
            if let Some(base) = *base {
                // If the expression uses FRU we need to make sure all the unmentioned fields
                // are checked for privacy (RFC 736). Rather than computing the set of
                // unmentioned fields, just check them all.
                for (vf_index, variant_field) in variant.fields.iter().enumerate() {
                    let field = fields.iter().find(|f| {
                        self.tcx.field_index(f.hir_id, self.typeck_results()) == vf_index
                    });
                    let (use_ctxt, span) = match field {
                        Some(field) => (field.ident.span, field.span),
                        None => (base.span, base.span),
                    };
                    self.check_field(use_ctxt, span, adt, variant_field, true);
                }
            } else {
                for field in fields {
                    let use_ctxt = field.ident.span;
                    let index = self.tcx.field_index(field.hir_id, self.typeck_results());
                    self.check_field(use_ctxt, field.span, adt, &variant.fields[index], false);
                }
            }
        }

        intravisit::walk_expr(self, expr);
    }

    fn visit_pat(&mut self, pat: &'tcx hir::Pat<'tcx>) {
        if let PatKind::Struct(ref qpath, fields, _) = pat.kind {
            let res = self.typeck_results().qpath_res(qpath, pat.hir_id);
            let adt = self.typeck_results().pat_ty(pat).ty_adt_def().unwrap();
            let variant = adt.variant_of_res(res);
            for field in fields {
                let use_ctxt = field.ident.span;
                let index = self.tcx.field_index(field.hir_id, self.typeck_results());
                self.check_field(use_ctxt, field.span, adt, &variant.fields[index], false);
            }
        }

        intravisit::walk_pat(self, pat);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
/// Type privacy visitor, checks types for privacy and reports violations.
/// Both explicitly written types and inferred types of expressions and patterns are checked.
/// Checks are performed on "semantic" types regardless of names and their hygiene.
////////////////////////////////////////////////////////////////////////////////////////////

struct TypePrivacyVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    maybe_typeck_results: Option<&'tcx ty::TypeckResults<'tcx>>,
    current_item: LocalDefId,
    span: Span,
}

impl<'tcx> TypePrivacyVisitor<'tcx> {
    /// Gets the type-checking results for the current body.
    /// As this will ICE if called outside bodies, only call when working with
    /// `Expr` or `Pat` nodes (they are guaranteed to be found only in bodies).
    #[track_caller]
    fn typeck_results(&self) -> &'tcx ty::TypeckResults<'tcx> {
        self.maybe_typeck_results
            .expect("`TypePrivacyVisitor::typeck_results` called outside of body")
    }

    fn item_is_accessible(&self, did: DefId) -> bool {
        self.tcx.visibility(did).is_accessible_from(self.current_item.to_def_id(), self.tcx)
    }

    // Take node-id of an expression or pattern and check its type for privacy.
    fn check_expr_pat_type(&mut self, id: hir::HirId, span: Span) -> bool {
        self.span = span;
        let typeck_results = self.typeck_results();
        let result: ControlFlow<()> = try {
            self.visit(typeck_results.node_type(id))?;
            self.visit(typeck_results.node_substs(id))?;
            if let Some(adjustments) = typeck_results.adjustments().get(id) {
                adjustments.iter().try_for_each(|adjustment| self.visit(adjustment.target))?;
            }
        };
        result.is_break()
    }

    fn check_def_id(&mut self, def_id: DefId, kind: &str, descr: &dyn fmt::Display) -> bool {
        let is_error = !self.item_is_accessible(def_id);
        if is_error {
            self.tcx
                .sess
                .struct_span_err(self.span, &format!("{} `{}` is private", kind, descr))
                .span_label(self.span, &format!("private {}", kind))
                .emit();
        }
        is_error
    }
}

impl<'tcx> Visitor<'tcx> for TypePrivacyVisitor<'tcx> {
    type Map = Map<'tcx>;

    /// We want to visit items in the context of their containing
    /// module and so forth, so supply a crate for doing a deep walk.
    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::All(self.tcx.hir())
    }

    fn visit_mod(&mut self, _m: &'tcx hir::Mod<'tcx>, _s: Span, _n: hir::HirId) {
        // Don't visit nested modules, since we run a separate visitor walk
        // for each module in `privacy_access_levels`
    }

    fn visit_nested_body(&mut self, body: hir::BodyId) {
        let old_maybe_typeck_results =
            self.maybe_typeck_results.replace(self.tcx.typeck_body(body));
        let body = self.tcx.hir().body(body);
        self.visit_body(body);
        self.maybe_typeck_results = old_maybe_typeck_results;
    }

    fn visit_generic_arg(&mut self, generic_arg: &'tcx hir::GenericArg<'tcx>) {
        match generic_arg {
            hir::GenericArg::Type(t) => self.visit_ty(t),
            hir::GenericArg::Infer(inf) => self.visit_infer(inf),
            hir::GenericArg::Lifetime(_) | hir::GenericArg::Const(_) => {}
        }
    }

    fn visit_ty(&mut self, hir_ty: &'tcx hir::Ty<'tcx>) {
        self.span = hir_ty.span;
        if let Some(typeck_results) = self.maybe_typeck_results {
            // Types in bodies.
            if self.visit(typeck_results.node_type(hir_ty.hir_id)).is_break() {
                return;
            }
        } else {
            // Types in signatures.
            // FIXME: This is very ineffective. Ideally each HIR type should be converted
            // into a semantic type only once and the result should be cached somehow.
            if self.visit(rustc_typeck::hir_ty_to_ty(self.tcx, hir_ty)).is_break() {
                return;
            }
        }

        intravisit::walk_ty(self, hir_ty);
    }

    fn visit_infer(&mut self, inf: &'tcx hir::InferArg) {
        self.span = inf.span;
        if let Some(typeck_results) = self.maybe_typeck_results {
            if let Some(ty) = typeck_results.node_type_opt(inf.hir_id) {
                if self.visit(ty).is_break() {
                    return;
                }
            }
        } else {
            let local_id = self.tcx.hir().local_def_id(inf.hir_id);
            if let Some(did) = self.tcx.opt_const_param_of(local_id) {
                if self.visit_def_id(did, "inferred", &"").is_break() {
                    return;
                }
            }

            // FIXME see above note for same issue.
            if self.visit(rustc_typeck::hir_ty_to_ty(self.tcx, &inf.to_ty())).is_break() {
                return;
            }
        }
        intravisit::walk_inf(self, inf);
    }

    fn visit_trait_ref(&mut self, trait_ref: &'tcx hir::TraitRef<'tcx>) {
        self.span = trait_ref.path.span;
        if self.maybe_typeck_results.is_none() {
            // Avoid calling `hir_trait_to_predicates` in bodies, it will ICE.
            // The traits' privacy in bodies is already checked as a part of trait object types.
            let bounds = rustc_typeck::hir_trait_to_predicates(
                self.tcx,
                trait_ref,
                // NOTE: This isn't really right, but the actual type doesn't matter here. It's
                // just required by `ty::TraitRef`.
                self.tcx.types.never,
            );

            for (trait_predicate, _, _) in bounds.trait_bounds {
                if self.visit_trait(trait_predicate.skip_binder()).is_break() {
                    return;
                }
            }

            for (poly_predicate, _) in bounds.projection_bounds {
                if self.visit(poly_predicate.skip_binder().ty).is_break()
                    || self
                        .visit_projection_ty(poly_predicate.skip_binder().projection_ty)
                        .is_break()
                {
                    return;
                }
            }
        }

        intravisit::walk_trait_ref(self, trait_ref);
    }

    // Check types of expressions
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if self.check_expr_pat_type(expr.hir_id, expr.span) {
            // Do not check nested expressions if the error already happened.
            return;
        }
        match expr.kind {
            hir::ExprKind::Assign(_, rhs, _) | hir::ExprKind::Match(rhs, ..) => {
                // Do not report duplicate errors for `x = y` and `match x { ... }`.
                if self.check_expr_pat_type(rhs.hir_id, rhs.span) {
                    return;
                }
            }
            hir::ExprKind::MethodCall(_, span, _, _) => {
                // Method calls have to be checked specially.
                self.span = span;
                if let Some(def_id) = self.typeck_results().type_dependent_def_id(expr.hir_id) {
                    if self.visit(self.tcx.type_of(def_id)).is_break() {
                        return;
                    }
                } else {
                    self.tcx
                        .sess
                        .delay_span_bug(expr.span, "no type-dependent def for method call");
                }
            }
            _ => {}
        }

        intravisit::walk_expr(self, expr);
    }

    // Prohibit access to associated items with insufficient nominal visibility.
    //
    // Additionally, until better reachability analysis for macros 2.0 is available,
    // we prohibit access to private statics from other crates, this allows to give
    // more code internal visibility at link time. (Access to private functions
    // is already prohibited by type privacy for function types.)
    fn visit_qpath(&mut self, qpath: &'tcx hir::QPath<'tcx>, id: hir::HirId, span: Span) {
        let def = match qpath {
            hir::QPath::Resolved(_, path) => match path.res {
                Res::Def(kind, def_id) => Some((kind, def_id)),
                _ => None,
            },
            hir::QPath::TypeRelative(..) | hir::QPath::LangItem(..) => self
                .maybe_typeck_results
                .and_then(|typeck_results| typeck_results.type_dependent_def(id)),
        };
        let def = def.filter(|(kind, _)| {
            matches!(
                kind,
                DefKind::AssocFn | DefKind::AssocConst | DefKind::AssocTy | DefKind::Static
            )
        });
        if let Some((kind, def_id)) = def {
            let is_local_static =
                if let DefKind::Static = kind { def_id.is_local() } else { false };
            if !self.item_is_accessible(def_id) && !is_local_static {
                let sess = self.tcx.sess;
                let sm = sess.source_map();
                let name = match qpath {
                    hir::QPath::Resolved(..) | hir::QPath::LangItem(..) => {
                        sm.span_to_snippet(qpath.span()).ok()
                    }
                    hir::QPath::TypeRelative(_, segment) => Some(segment.ident.to_string()),
                };
                let kind = kind.descr(def_id);
                let msg = match name {
                    Some(name) => format!("{} `{}` is private", kind, name),
                    None => format!("{} is private", kind),
                };
                sess.struct_span_err(span, &msg)
                    .span_label(span, &format!("private {}", kind))
                    .emit();
                return;
            }
        }

        intravisit::walk_qpath(self, qpath, id, span);
    }

    // Check types of patterns.
    fn visit_pat(&mut self, pattern: &'tcx hir::Pat<'tcx>) {
        if self.check_expr_pat_type(pattern.hir_id, pattern.span) {
            // Do not check nested patterns if the error already happened.
            return;
        }

        intravisit::walk_pat(self, pattern);
    }

    fn visit_local(&mut self, local: &'tcx hir::Local<'tcx>) {
        if let Some(init) = local.init {
            if self.check_expr_pat_type(init.hir_id, init.span) {
                // Do not report duplicate errors for `let x = y`.
                return;
            }
        }

        intravisit::walk_local(self, local);
    }

    // Check types in item interfaces.
    fn visit_item(&mut self, item: &'tcx hir::Item<'tcx>) {
        let orig_current_item = mem::replace(&mut self.current_item, item.def_id);
        let old_maybe_typeck_results = self.maybe_typeck_results.take();
        intravisit::walk_item(self, item);
        self.maybe_typeck_results = old_maybe_typeck_results;
        self.current_item = orig_current_item;
    }
}

impl DefIdVisitor<'tcx> for TypePrivacyVisitor<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }
    fn visit_def_id(
        &mut self,
        def_id: DefId,
        kind: &str,
        descr: &dyn fmt::Display,
    ) -> ControlFlow<Self::BreakTy> {
        if self.check_def_id(def_id, kind, descr) {
            ControlFlow::BREAK
        } else {
            ControlFlow::CONTINUE
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// Obsolete visitors for checking for private items in public interfaces.
/// These visitors are supposed to be kept in frozen state and produce an
/// "old error node set". For backward compatibility the new visitor reports
/// warnings instead of hard errors when the erroneous node is not in this old set.
///////////////////////////////////////////////////////////////////////////////

struct ObsoleteVisiblePrivateTypesVisitor<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    access_levels: &'a AccessLevels,
    in_variant: bool,
    // Set of errors produced by this obsolete visitor.
    old_error_set: HirIdSet,
}

struct ObsoleteCheckTypeForPrivatenessVisitor<'a, 'b, 'tcx> {
    inner: &'a ObsoleteVisiblePrivateTypesVisitor<'b, 'tcx>,
    /// Whether the type refers to private types.
    contains_private: bool,
    /// Whether we've recurred at all (i.e., if we're pointing at the
    /// first type on which `visit_ty` was called).
    at_outer_type: bool,
    /// Whether that first type is a public path.
    outer_type_is_public_path: bool,
}

impl<'a, 'tcx> ObsoleteVisiblePrivateTypesVisitor<'a, 'tcx> {
    fn path_is_private_type(&self, path: &hir::Path<'_>) -> bool {
        let did = match path.res {
            Res::PrimTy(..) | Res::SelfTy(..) | Res::Err => return false,
            res => res.def_id(),
        };

        // A path can only be private if:
        // it's in this crate...
        if let Some(did) = did.as_local() {
            // .. and it corresponds to a private type in the AST (this returns
            // `None` for type parameters).
            match self.tcx.hir().find(self.tcx.hir().local_def_id_to_hir_id(did)) {
                Some(Node::Item(item)) => !item.vis.node.is_pub(),
                Some(_) | None => false,
            }
        } else {
            false
        }
    }

    fn trait_is_public(&self, trait_id: LocalDefId) -> bool {
        // FIXME: this would preferably be using `exported_items`, but all
        // traits are exported currently (see `EmbargoVisitor.exported_trait`).
        self.access_levels.is_public(trait_id)
    }

    fn check_generic_bound(&mut self, bound: &hir::GenericBound<'_>) {
        if let hir::GenericBound::Trait(ref trait_ref, _) = *bound {
            if self.path_is_private_type(trait_ref.trait_ref.path) {
                self.old_error_set.insert(trait_ref.trait_ref.hir_ref_id);
            }
        }
    }

    fn item_is_public(&self, def_id: LocalDefId, vis: &hir::Visibility<'_>) -> bool {
        self.access_levels.is_reachable(def_id) || vis.node.is_pub()
    }
}

impl<'a, 'b, 'tcx, 'v> Visitor<'v> for ObsoleteCheckTypeForPrivatenessVisitor<'a, 'b, 'tcx> {
    type Map = intravisit::ErasedMap<'v>;

    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::None
    }

    fn visit_generic_arg(&mut self, generic_arg: &'v hir::GenericArg<'v>) {
        match generic_arg {
            hir::GenericArg::Type(t) => self.visit_ty(t),
            hir::GenericArg::Infer(inf) => self.visit_ty(&inf.to_ty()),
            hir::GenericArg::Lifetime(_) | hir::GenericArg::Const(_) => {}
        }
    }

    fn visit_ty(&mut self, ty: &hir::Ty<'_>) {
        if let hir::TyKind::Path(hir::QPath::Resolved(_, path)) = ty.kind {
            if self.inner.path_is_private_type(path) {
                self.contains_private = true;
                // Found what we're looking for, so let's stop working.
                return;
            }
        }
        if let hir::TyKind::Path(_) = ty.kind {
            if self.at_outer_type {
                self.outer_type_is_public_path = true;
            }
        }
        self.at_outer_type = false;
        intravisit::walk_ty(self, ty)
    }

    // Don't want to recurse into `[, .. expr]`.
    fn visit_expr(&mut self, _: &hir::Expr<'_>) {}
}

impl<'a, 'tcx> Visitor<'tcx> for ObsoleteVisiblePrivateTypesVisitor<'a, 'tcx> {
    type Map = Map<'tcx>;

    /// We want to visit items in the context of their containing
    /// module and so forth, so supply a crate for doing a deep walk.
    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::All(self.tcx.hir())
    }

    fn visit_item(&mut self, item: &'tcx hir::Item<'tcx>) {
        match item.kind {
            // Contents of a private mod can be re-exported, so we need
            // to check internals.
            hir::ItemKind::Mod(_) => {}

            // An `extern {}` doesn't introduce a new privacy
            // namespace (the contents have their own privacies).
            hir::ItemKind::ForeignMod { .. } => {}

            hir::ItemKind::Trait(.., bounds, _) => {
                if !self.trait_is_public(item.def_id) {
                    return;
                }

                for bound in bounds.iter() {
                    self.check_generic_bound(bound)
                }
            }

            // Impls need some special handling to try to offer useful
            // error messages without (too many) false positives
            // (i.e., we could just return here to not check them at
            // all, or some worse estimation of whether an impl is
            // publicly visible).
            hir::ItemKind::Impl(ref impl_) => {
                // `impl [... for] Private` is never visible.
                let self_contains_private;
                // `impl [... for] Public<...>`, but not `impl [... for]
                // Vec<Public>` or `(Public,)`, etc.
                let self_is_public_path;

                // Check the properties of the `Self` type:
                {
                    let mut visitor = ObsoleteCheckTypeForPrivatenessVisitor {
                        inner: self,
                        contains_private: false,
                        at_outer_type: true,
                        outer_type_is_public_path: false,
                    };
                    visitor.visit_ty(impl_.self_ty);
                    self_contains_private = visitor.contains_private;
                    self_is_public_path = visitor.outer_type_is_public_path;
                }

                // Miscellaneous info about the impl:

                // `true` iff this is `impl Private for ...`.
                let not_private_trait = impl_.of_trait.as_ref().map_or(
                    true, // no trait counts as public trait
                    |tr| {
                        if let Some(def_id) = tr.path.res.def_id().as_local() {
                            self.trait_is_public(def_id)
                        } else {
                            true // external traits must be public
                        }
                    },
                );

                // `true` iff this is a trait impl or at least one method is public.
                //
                // `impl Public { $( fn ...() {} )* }` is not visible.
                //
                // This is required over just using the methods' privacy
                // directly because we might have `impl<T: Foo<Private>> ...`,
                // and we shouldn't warn about the generics if all the methods
                // are private (because `T` won't be visible externally).
                let trait_or_some_public_method = impl_.of_trait.is_some()
                    || impl_.items.iter().any(|impl_item_ref| {
                        let impl_item = self.tcx.hir().impl_item(impl_item_ref.id);
                        match impl_item.kind {
                            hir::ImplItemKind::Const(..) | hir::ImplItemKind::Fn(..) => {
                                self.access_levels.is_reachable(impl_item_ref.id.def_id)
                            }
                            hir::ImplItemKind::TyAlias(_) => false,
                        }
                    });

                if !self_contains_private && not_private_trait && trait_or_some_public_method {
                    intravisit::walk_generics(self, &impl_.generics);

                    match impl_.of_trait {
                        None => {
                            for impl_item_ref in impl_.items {
                                // This is where we choose whether to walk down
                                // further into the impl to check its items. We
                                // should only walk into public items so that we
                                // don't erroneously report errors for private
                                // types in private items.
                                let impl_item = self.tcx.hir().impl_item(impl_item_ref.id);
                                match impl_item.kind {
                                    hir::ImplItemKind::Const(..) | hir::ImplItemKind::Fn(..)
                                        if self
                                            .item_is_public(impl_item.def_id, &impl_item.vis) =>
                                    {
                                        intravisit::walk_impl_item(self, impl_item)
                                    }
                                    hir::ImplItemKind::TyAlias(..) => {
                                        intravisit::walk_impl_item(self, impl_item)
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(ref tr) => {
                            // Any private types in a trait impl fall into three
                            // categories.
                            // 1. mentioned in the trait definition
                            // 2. mentioned in the type params/generics
                            // 3. mentioned in the associated types of the impl
                            //
                            // Those in 1. can only occur if the trait is in
                            // this crate and will've been warned about on the
                            // trait definition (there's no need to warn twice
                            // so we don't check the methods).
                            //
                            // Those in 2. are warned via walk_generics and this
                            // call here.
                            intravisit::walk_path(self, tr.path);

                            // Those in 3. are warned with this call.
                            for impl_item_ref in impl_.items {
                                let impl_item = self.tcx.hir().impl_item(impl_item_ref.id);
                                if let hir::ImplItemKind::TyAlias(ty) = impl_item.kind {
                                    self.visit_ty(ty);
                                }
                            }
                        }
                    }
                } else if impl_.of_trait.is_none() && self_is_public_path {
                    // `impl Public<Private> { ... }`. Any public static
                    // methods will be visible as `Public::foo`.
                    let mut found_pub_static = false;
                    for impl_item_ref in impl_.items {
                        if self.access_levels.is_reachable(impl_item_ref.id.def_id)
                            || self.tcx.visibility(impl_item_ref.id.def_id)
                                == ty::Visibility::Public
                        {
                            let impl_item = self.tcx.hir().impl_item(impl_item_ref.id);
                            match impl_item_ref.kind {
                                AssocItemKind::Const => {
                                    found_pub_static = true;
                                    intravisit::walk_impl_item(self, impl_item);
                                }
                                AssocItemKind::Fn { has_self: false } => {
                                    found_pub_static = true;
                                    intravisit::walk_impl_item(self, impl_item);
                                }
                                _ => {}
                            }
                        }
                    }
                    if found_pub_static {
                        intravisit::walk_generics(self, &impl_.generics)
                    }
                }
                return;
            }

            // `type ... = ...;` can contain private types, because
            // we're introducing a new name.
            hir::ItemKind::TyAlias(..) => return,

            // Not at all public, so we don't care.
            _ if !self.item_is_public(item.def_id, &item.vis) => {
                return;
            }

            _ => {}
        }

        // We've carefully constructed it so that if we're here, then
        // any `visit_ty`'s will be called on things that are in
        // public signatures, i.e., things that we're interested in for
        // this visitor.
        intravisit::walk_item(self, item);
    }

    fn visit_generics(&mut self, generics: &'tcx hir::Generics<'tcx>) {
        for param in generics.params {
            for bound in param.bounds {
                self.check_generic_bound(bound);
            }
        }
        for predicate in generics.where_clause.predicates {
            match predicate {
                hir::WherePredicate::BoundPredicate(bound_pred) => {
                    for bound in bound_pred.bounds.iter() {
                        self.check_generic_bound(bound)
                    }
                }
                hir::WherePredicate::RegionPredicate(_) => {}
                hir::WherePredicate::EqPredicate(eq_pred) => {
                    self.visit_ty(eq_pred.rhs_ty);
                }
            }
        }
    }

    fn visit_foreign_item(&mut self, item: &'tcx hir::ForeignItem<'tcx>) {
        if self.access_levels.is_reachable(item.def_id) {
            intravisit::walk_foreign_item(self, item)
        }
    }

    fn visit_ty(&mut self, t: &'tcx hir::Ty<'tcx>) {
        if let hir::TyKind::Path(hir::QPath::Resolved(_, path)) = t.kind {
            if self.path_is_private_type(path) {
                self.old_error_set.insert(t.hir_id);
            }
        }
        intravisit::walk_ty(self, t)
    }

    fn visit_variant(
        &mut self,
        v: &'tcx hir::Variant<'tcx>,
        g: &'tcx hir::Generics<'tcx>,
        item_id: hir::HirId,
    ) {
        if self.access_levels.is_reachable(self.tcx.hir().local_def_id(v.id)) {
            self.in_variant = true;
            intravisit::walk_variant(self, v, g, item_id);
            self.in_variant = false;
        }
    }

    fn visit_field_def(&mut self, s: &'tcx hir::FieldDef<'tcx>) {
        if s.vis.node.is_pub() || self.in_variant {
            intravisit::walk_field_def(self, s);
        }
    }

    // We don't need to introspect into these at all: an
    // expression/block context can't possibly contain exported things.
    // (Making them no-ops stops us from traversing the whole AST without
    // having to be super careful about our `walk_...` calls above.)
    fn visit_block(&mut self, _: &'tcx hir::Block<'tcx>) {}
    fn visit_expr(&mut self, _: &'tcx hir::Expr<'tcx>) {}
}

///////////////////////////////////////////////////////////////////////////////
/// SearchInterfaceForPrivateItemsVisitor traverses an item's interface and
/// finds any private components in it.
/// PrivateItemsInPublicInterfacesVisitor ensures there are no private types
/// and traits in public interfaces.
///////////////////////////////////////////////////////////////////////////////

struct SearchInterfaceForPrivateItemsVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    item_def_id: LocalDefId,
    /// The visitor checks that each component type is at least this visible.
    required_visibility: ty::Visibility,
    has_pub_restricted: bool,
    has_old_errors: bool,
    in_assoc_ty: bool,
}

impl SearchInterfaceForPrivateItemsVisitor<'tcx> {
    fn generics(&mut self) -> &mut Self {
        for param in &self.tcx.generics_of(self.item_def_id).params {
            match param.kind {
                GenericParamDefKind::Lifetime => {}
                GenericParamDefKind::Type { has_default, .. } => {
                    if has_default {
                        self.visit(self.tcx.type_of(param.def_id));
                    }
                }
                // FIXME(generic_const_exprs): May want to look inside const here
                GenericParamDefKind::Const { .. } => {
                    self.visit(self.tcx.type_of(param.def_id));
                }
            }
        }
        self
    }

    fn predicates(&mut self) -> &mut Self {
        // N.B., we use `explicit_predicates_of` and not `predicates_of`
        // because we don't want to report privacy errors due to where
        // clauses that the compiler inferred. We only want to
        // consider the ones that the user wrote. This is important
        // for the inferred outlives rules; see
        // `src/test/ui/rfc-2093-infer-outlives/privacy.rs`.
        self.visit_predicates(self.tcx.explicit_predicates_of(self.item_def_id));
        self
    }

    fn bounds(&mut self) -> &mut Self {
        self.visit_predicates(ty::GenericPredicates {
            parent: None,
            predicates: self.tcx.explicit_item_bounds(self.item_def_id),
        });
        self
    }

    fn ty(&mut self) -> &mut Self {
        self.visit(self.tcx.type_of(self.item_def_id));
        self
    }

    fn check_def_id(&mut self, def_id: DefId, kind: &str, descr: &dyn fmt::Display) -> bool {
        if self.leaks_private_dep(def_id) {
            self.tcx.struct_span_lint_hir(
                lint::builtin::EXPORTED_PRIVATE_DEPENDENCIES,
                self.tcx.hir().local_def_id_to_hir_id(self.item_def_id),
                self.tcx.def_span(self.item_def_id.to_def_id()),
                |lint| {
                    lint.build(&format!(
                        "{} `{}` from private dependency '{}' in public \
                                                interface",
                        kind,
                        descr,
                        self.tcx.crate_name(def_id.krate)
                    ))
                    .emit()
                },
            );
        }

        let hir_id = match def_id.as_local() {
            Some(def_id) => self.tcx.hir().local_def_id_to_hir_id(def_id),
            None => return false,
        };

        let vis = self.tcx.visibility(def_id);
        if !vis.is_at_least(self.required_visibility, self.tcx) {
            let vis_descr = match vis {
                ty::Visibility::Public => "public",
                ty::Visibility::Invisible => "private",
                ty::Visibility::Restricted(vis_def_id) => {
                    if vis_def_id == self.tcx.parent_module(hir_id).to_def_id() {
                        "private"
                    } else if vis_def_id.is_top_level_module() {
                        "crate-private"
                    } else {
                        "restricted"
                    }
                }
            };
            let make_msg = || format!("{} {} `{}` in public interface", vis_descr, kind, descr);
            let span = self.tcx.def_span(self.item_def_id.to_def_id());
            if self.has_pub_restricted || self.has_old_errors || self.in_assoc_ty {
                let mut err = if kind == "trait" {
                    struct_span_err!(self.tcx.sess, span, E0445, "{}", make_msg())
                } else {
                    struct_span_err!(self.tcx.sess, span, E0446, "{}", make_msg())
                };
                let vis_span =
                    self.tcx.sess.source_map().guess_head_span(self.tcx.def_span(def_id));
                err.span_label(span, format!("can't leak {} {}", vis_descr, kind));
                err.span_label(vis_span, format!("`{}` declared as {}", descr, vis_descr));
                err.emit();
            } else {
                let err_code = if kind == "trait" { "E0445" } else { "E0446" };
                self.tcx.struct_span_lint_hir(
                    lint::builtin::PRIVATE_IN_PUBLIC,
                    hir_id,
                    span,
                    |lint| lint.build(&format!("{} (error {})", make_msg(), err_code)).emit(),
                );
            }
        }

        false
    }

    /// An item is 'leaked' from a private dependency if all
    /// of the following are true:
    /// 1. It's contained within a public type
    /// 2. It comes from a private crate
    fn leaks_private_dep(&self, item_id: DefId) -> bool {
        let ret = self.required_visibility == ty::Visibility::Public
            && self.tcx.is_private_dep(item_id.krate);

        tracing::debug!("leaks_private_dep(item_id={:?})={}", item_id, ret);
        ret
    }
}

impl DefIdVisitor<'tcx> for SearchInterfaceForPrivateItemsVisitor<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }
    fn visit_def_id(
        &mut self,
        def_id: DefId,
        kind: &str,
        descr: &dyn fmt::Display,
    ) -> ControlFlow<Self::BreakTy> {
        if self.check_def_id(def_id, kind, descr) {
            ControlFlow::BREAK
        } else {
            ControlFlow::CONTINUE
        }
    }
}

struct PrivateItemsInPublicInterfacesVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    has_pub_restricted: bool,
    old_error_set_ancestry: LocalDefIdSet,
}

impl<'tcx> PrivateItemsInPublicInterfacesVisitor<'tcx> {
    fn check(
        &self,
        def_id: LocalDefId,
        required_visibility: ty::Visibility,
    ) -> SearchInterfaceForPrivateItemsVisitor<'tcx> {
        SearchInterfaceForPrivateItemsVisitor {
            tcx: self.tcx,
            item_def_id: def_id,
            required_visibility,
            has_pub_restricted: self.has_pub_restricted,
            has_old_errors: self.old_error_set_ancestry.contains(&def_id),
            in_assoc_ty: false,
        }
    }

    fn check_assoc_item(
        &self,
        def_id: LocalDefId,
        assoc_item_kind: AssocItemKind,
        defaultness: hir::Defaultness,
        vis: ty::Visibility,
    ) {
        let mut check = self.check(def_id, vis);

        let (check_ty, is_assoc_ty) = match assoc_item_kind {
            AssocItemKind::Const | AssocItemKind::Fn { .. } => (true, false),
            AssocItemKind::Type => (defaultness.has_value(), true),
        };
        check.in_assoc_ty = is_assoc_ty;
        check.generics().predicates();
        if check_ty {
            check.ty();
        }
    }
}

impl<'tcx> Visitor<'tcx> for PrivateItemsInPublicInterfacesVisitor<'tcx> {
    type Map = Map<'tcx>;

    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::OnlyBodies(self.tcx.hir())
    }

    fn visit_item(&mut self, item: &'tcx hir::Item<'tcx>) {
        let tcx = self.tcx;
        let item_visibility = tcx.visibility(item.def_id);

        match item.kind {
            // Crates are always public.
            hir::ItemKind::ExternCrate(..) => {}
            // All nested items are checked by `visit_item`.
            hir::ItemKind::Mod(..) => {}
            // Checked in resolve.
            hir::ItemKind::Use(..) => {}
            // No subitems.
            hir::ItemKind::Macro(..) | hir::ItemKind::GlobalAsm(..) => {}
            // Subitems of these items have inherited publicity.
            hir::ItemKind::Const(..)
            | hir::ItemKind::Static(..)
            | hir::ItemKind::Fn(..)
            | hir::ItemKind::TyAlias(..) => {
                self.check(item.def_id, item_visibility).generics().predicates().ty();
            }
            hir::ItemKind::OpaqueTy(..) => {
                // `ty()` for opaque types is the underlying type,
                // it's not a part of interface, so we skip it.
                self.check(item.def_id, item_visibility).generics().bounds();
            }
            hir::ItemKind::Trait(.., trait_item_refs) => {
                self.check(item.def_id, item_visibility).generics().predicates();

                for trait_item_ref in trait_item_refs {
                    self.check_assoc_item(
                        trait_item_ref.id.def_id,
                        trait_item_ref.kind,
                        trait_item_ref.defaultness,
                        item_visibility,
                    );

                    if let AssocItemKind::Type = trait_item_ref.kind {
                        self.check(trait_item_ref.id.def_id, item_visibility).bounds();
                    }
                }
            }
            hir::ItemKind::TraitAlias(..) => {
                self.check(item.def_id, item_visibility).generics().predicates();
            }
            hir::ItemKind::Enum(ref def, _) => {
                self.check(item.def_id, item_visibility).generics().predicates();

                for variant in def.variants {
                    for field in variant.data.fields() {
                        self.check(self.tcx.hir().local_def_id(field.hir_id), item_visibility).ty();
                    }
                }
            }
            // Subitems of foreign modules have their own publicity.
            hir::ItemKind::ForeignMod { items, .. } => {
                for foreign_item in items {
                    let vis = tcx.visibility(foreign_item.id.def_id);
                    self.check(foreign_item.id.def_id, vis).generics().predicates().ty();
                }
            }
            // Subitems of structs and unions have their own publicity.
            hir::ItemKind::Struct(ref struct_def, _) | hir::ItemKind::Union(ref struct_def, _) => {
                self.check(item.def_id, item_visibility).generics().predicates();

                for field in struct_def.fields() {
                    let def_id = tcx.hir().local_def_id(field.hir_id);
                    let field_visibility = tcx.visibility(def_id);
                    self.check(def_id, min(item_visibility, field_visibility, tcx)).ty();
                }
            }
            // An inherent impl is public when its type is public
            // Subitems of inherent impls have their own publicity.
            // A trait impl is public when both its type and its trait are public
            // Subitems of trait impls have inherited publicity.
            hir::ItemKind::Impl(ref impl_) => {
                let impl_vis = ty::Visibility::of_impl(item.def_id, tcx, &Default::default());
                self.check(item.def_id, impl_vis).generics().predicates();
                for impl_item_ref in impl_.items {
                    let impl_item_vis = if impl_.of_trait.is_none() {
                        min(tcx.visibility(impl_item_ref.id.def_id), impl_vis, tcx)
                    } else {
                        impl_vis
                    };
                    self.check_assoc_item(
                        impl_item_ref.id.def_id,
                        impl_item_ref.kind,
                        impl_item_ref.defaultness,
                        impl_item_vis,
                    );
                }
            }
        }
    }
}

pub fn provide(providers: &mut Providers) {
    *providers = Providers {
        visibility,
        privacy_access_levels,
        check_private_in_public,
        check_mod_privacy,
        ..*providers
    };
}

fn visibility(tcx: TyCtxt<'_>, def_id: DefId) -> ty::Visibility {
    let def_id = def_id.expect_local();
    match tcx.resolutions(()).visibilities.get(&def_id) {
        Some(vis) => *vis,
        None => {
            let hir_id = tcx.hir().local_def_id_to_hir_id(def_id);
            match tcx.hir().get(hir_id) {
                // Unique types created for closures participate in type privacy checking.
                // They have visibilities inherited from the module they are defined in.
                Node::Expr(hir::Expr { kind: hir::ExprKind::Closure(..), .. }) => {
                    ty::Visibility::Restricted(tcx.parent_module(hir_id).to_def_id())
                }
                // - AST lowering may clone `use` items and the clones don't
                //   get their entries in the resolver's visibility table.
                // - AST lowering also creates opaque type items with inherited visibilies.
                //   Visibility on them should have no effect, but to avoid the visibility
                //   query failing on some items, we provide it for opaque types as well.
                Node::Item(hir::Item {
                    vis,
                    kind: hir::ItemKind::Use(..) | hir::ItemKind::OpaqueTy(..),
                    ..
                }) => ty::Visibility::from_hir(vis, hir_id, tcx),
                // Visibilities of trait impl items are inherited from their traits
                // and are not filled in resolve.
                Node::ImplItem(impl_item) => {
                    match tcx.hir().get(tcx.hir().get_parent_item(hir_id)) {
                        Node::Item(hir::Item {
                            kind: hir::ItemKind::Impl(hir::Impl { of_trait: Some(tr), .. }),
                            ..
                        }) => tr.path.res.opt_def_id().map_or_else(
                            || {
                                tcx.sess.delay_span_bug(tr.path.span, "trait without a def-id");
                                ty::Visibility::Public
                            },
                            |def_id| tcx.visibility(def_id),
                        ),
                        _ => span_bug!(impl_item.span, "the parent is not a trait impl"),
                    }
                }
                _ => span_bug!(
                    tcx.def_span(def_id),
                    "visibility table unexpectedly missing a def-id: {:?}",
                    def_id,
                ),
            }
        }
    }
}

fn check_mod_privacy(tcx: TyCtxt<'_>, module_def_id: LocalDefId) {
    // Check privacy of names not checked in previous compilation stages.
    let mut visitor =
        NamePrivacyVisitor { tcx, maybe_typeck_results: None, current_item: module_def_id };
    let (module, span, hir_id) = tcx.hir().get_module(module_def_id);

    intravisit::walk_mod(&mut visitor, module, hir_id);

    // Check privacy of explicitly written types and traits as well as
    // inferred types of expressions and patterns.
    let mut visitor =
        TypePrivacyVisitor { tcx, maybe_typeck_results: None, current_item: module_def_id, span };
    intravisit::walk_mod(&mut visitor, module, hir_id);
}

fn privacy_access_levels(tcx: TyCtxt<'_>, (): ()) -> &AccessLevels {
    // Build up a set of all exported items in the AST. This is a set of all
    // items which are reachable from external crates based on visibility.
    let mut visitor = EmbargoVisitor {
        tcx,
        access_levels: Default::default(),
        macro_reachable: Default::default(),
        prev_level: Some(AccessLevel::Public),
        changed: false,
    };
    loop {
        tcx.hir().walk_toplevel_module(&mut visitor);
        if visitor.changed {
            visitor.changed = false;
        } else {
            break;
        }
    }
    visitor.update(CRATE_DEF_ID, Some(AccessLevel::Public));

    tcx.arena.alloc(visitor.access_levels)
}

fn check_private_in_public(tcx: TyCtxt<'_>, (): ()) {
    let access_levels = tcx.privacy_access_levels(());

    let mut visitor = ObsoleteVisiblePrivateTypesVisitor {
        tcx,
        access_levels,
        in_variant: false,
        old_error_set: Default::default(),
    };
    tcx.hir().walk_toplevel_module(&mut visitor);

    let has_pub_restricted = {
        let mut pub_restricted_visitor = PubRestrictedVisitor { tcx, has_pub_restricted: false };
        tcx.hir().walk_toplevel_module(&mut pub_restricted_visitor);
        pub_restricted_visitor.has_pub_restricted
    };

    let mut old_error_set_ancestry = HirIdSet::default();
    for mut id in visitor.old_error_set.iter().copied() {
        loop {
            if !old_error_set_ancestry.insert(id) {
                break;
            }
            let parent = tcx.hir().get_parent_node(id);
            if parent == id {
                break;
            }
            id = parent;
        }
    }

    // Check for private types and traits in public interfaces.
    let mut visitor = PrivateItemsInPublicInterfacesVisitor {
        tcx,
        has_pub_restricted,
        // Only definition IDs are ever searched in `old_error_set_ancestry`,
        // so we can filter away all non-definition IDs at this point.
        old_error_set_ancestry: old_error_set_ancestry
            .into_iter()
            .filter_map(|hir_id| tcx.hir().opt_local_def_id(hir_id))
            .collect(),
    };
    tcx.hir().visit_all_item_likes(&mut DeepVisitor::new(&mut visitor));
}
