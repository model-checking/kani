use rustc_errors::{Applicability, ErrorReported, StashKey};
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit;
use rustc_hir::intravisit::Visitor;
use rustc_hir::{HirId, Node};
use rustc_middle::hir::map::Map;
use rustc_middle::ty::subst::{InternalSubsts, SubstsRef};
use rustc_middle::ty::util::IntTypeExt;
use rustc_middle::ty::{self, DefIdTree, Ty, TyCtxt, TypeFoldable, TypeFolder};
use rustc_span::symbol::Ident;
use rustc_span::{Span, DUMMY_SP};

use super::ItemCtxt;
use super::{bad_placeholder, is_suggestable_infer_ty};

/// Computes the relevant generic parameter for a potential generic const argument.
///
/// This should be called using the query `tcx.opt_const_param_of`.
pub(super) fn opt_const_param_of(tcx: TyCtxt<'_>, def_id: LocalDefId) -> Option<DefId> {
    // FIXME(generic_arg_infer): allow for returning DefIds of inference of
    // GenericArg::Infer below. This may require a change where GenericArg::Infer has some flag
    // for const or type.
    use hir::*;
    let hir_id = tcx.hir().local_def_id_to_hir_id(def_id);

    if let Node::AnonConst(_) = tcx.hir().get(hir_id) {
        let parent_node_id = tcx.hir().get_parent_node(hir_id);
        let parent_node = tcx.hir().get(parent_node_id);

        match parent_node {
            // This match arm is for when the def_id appears in a GAT whose
            // path can't be resolved without typechecking e.g.
            //
            // trait Foo {
            //   type Assoc<const N: usize>;
            //   fn foo() -> Self::Assoc<3>;
            // }
            //
            // In the above code we would call this query with the def_id of 3 and
            // the parent_node we match on would be the hir node for Self::Assoc<3>
            //
            // `Self::Assoc<3>` cant be resolved without typchecking here as we
            // didnt write <Self as Foo>::Assoc<3>. If we did then another match
            // arm would handle this.
            //
            // I believe this match arm is only needed for GAT but I am not 100% sure - BoxyUwU
            Node::Ty(hir_ty @ Ty { kind: TyKind::Path(QPath::TypeRelative(_, segment)), .. }) => {
                // Find the Item containing the associated type so we can create an ItemCtxt.
                // Using the ItemCtxt convert the HIR for the unresolved assoc type into a
                // ty which is a fully resolved projection.
                // For the code example above, this would mean converting Self::Assoc<3>
                // into a ty::Projection(<Self as Foo>::Assoc<3>)
                let item_hir_id = tcx
                    .hir()
                    .parent_iter(hir_id)
                    .filter(|(_, node)| matches!(node, Node::Item(_)))
                    .map(|(id, _)| id)
                    .next()
                    .unwrap();
                let item_did = tcx.hir().local_def_id(item_hir_id).to_def_id();
                let item_ctxt = &ItemCtxt::new(tcx, item_did) as &dyn crate::astconv::AstConv<'_>;
                let ty = item_ctxt.ast_ty_to_ty(hir_ty);

                // Iterate through the generics of the projection to find the one that corresponds to
                // the def_id that this query was called with. We filter to only const args here as a
                // precaution for if it's ever allowed to elide lifetimes in GAT's. It currently isn't
                // but it can't hurt to be safe ^^
                if let ty::Projection(projection) = ty.kind() {
                    let generics = tcx.generics_of(projection.item_def_id);

                    let arg_index = segment
                        .args
                        .and_then(|args| {
                            args.args
                                .iter()
                                .filter(|arg| arg.is_const())
                                .position(|arg| arg.id() == hir_id)
                        })
                        .unwrap_or_else(|| {
                            bug!("no arg matching AnonConst in segment");
                        });

                    return generics
                        .params
                        .iter()
                        .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Const { .. }))
                        .nth(arg_index)
                        .map(|param| param.def_id);
                }

                // I dont think it's possible to reach this but I'm not 100% sure - BoxyUwU
                tcx.sess.delay_span_bug(
                    tcx.def_span(def_id),
                    "unexpected non-GAT usage of an anon const",
                );
                return None;
            }
            Node::Expr(&Expr {
                kind:
                    ExprKind::MethodCall(segment, ..) | ExprKind::Path(QPath::TypeRelative(_, segment)),
                ..
            }) => {
                let body_owner = tcx.hir().local_def_id(tcx.hir().enclosing_body_owner(hir_id));
                let tables = tcx.typeck(body_owner);
                // This may fail in case the method/path does not actually exist.
                // As there is no relevant param for `def_id`, we simply return
                // `None` here.
                let type_dependent_def = tables.type_dependent_def_id(parent_node_id)?;
                let idx = segment
                    .args
                    .and_then(|args| {
                        args.args
                            .iter()
                            .filter(|arg| arg.is_const())
                            .position(|arg| arg.id() == hir_id)
                    })
                    .unwrap_or_else(|| {
                        bug!("no arg matching AnonConst in segment");
                    });

                tcx.generics_of(type_dependent_def)
                    .params
                    .iter()
                    .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Const { .. }))
                    .nth(idx)
                    .map(|param| param.def_id)
            }

            Node::Ty(&Ty { kind: TyKind::Path(_), .. })
            | Node::Expr(&Expr { kind: ExprKind::Path(_) | ExprKind::Struct(..), .. })
            | Node::TraitRef(..)
            | Node::Pat(_) => {
                let path = match parent_node {
                    Node::Ty(&Ty { kind: TyKind::Path(QPath::Resolved(_, path)), .. })
                    | Node::TraitRef(&TraitRef { path, .. }) => &*path,
                    Node::Expr(&Expr {
                        kind:
                            ExprKind::Path(QPath::Resolved(_, path))
                            | ExprKind::Struct(&QPath::Resolved(_, path), ..),
                        ..
                    }) => {
                        let body_owner =
                            tcx.hir().local_def_id(tcx.hir().enclosing_body_owner(hir_id));
                        let _tables = tcx.typeck(body_owner);
                        &*path
                    }
                    Node::Pat(pat) => {
                        if let Some(path) = get_path_containing_arg_in_pat(pat, hir_id) {
                            path
                        } else {
                            tcx.sess.delay_span_bug(
                                tcx.def_span(def_id),
                                &format!(
                                    "unable to find const parent for {} in pat {:?}",
                                    hir_id, pat
                                ),
                            );
                            return None;
                        }
                    }
                    _ => {
                        tcx.sess.delay_span_bug(
                            tcx.def_span(def_id),
                            &format!("unexpected const parent path {:?}", parent_node),
                        );
                        return None;
                    }
                };

                // We've encountered an `AnonConst` in some path, so we need to
                // figure out which generic parameter it corresponds to and return
                // the relevant type.
                let filtered = path
                    .segments
                    .iter()
                    .filter_map(|seg| seg.args.map(|args| (args.args, seg)))
                    .find_map(|(args, seg)| {
                        args.iter()
                            .filter(|arg| arg.is_const())
                            .position(|arg| arg.id() == hir_id)
                            .map(|index| (index, seg))
                    });
                let (arg_index, segment) = match filtered {
                    None => {
                        tcx.sess.delay_span_bug(
                            tcx.def_span(def_id),
                            "no arg matching AnonConst in path",
                        );
                        return None;
                    }
                    Some(inner) => inner,
                };

                // Try to use the segment resolution if it is valid, otherwise we
                // default to the path resolution.
                let res = segment.res.filter(|&r| r != Res::Err).unwrap_or(path.res);
                use def::CtorOf;
                let generics = match res {
                    Res::Def(DefKind::Ctor(CtorOf::Variant, _), def_id) => tcx.generics_of(
                        tcx.parent(def_id).and_then(|def_id| tcx.parent(def_id)).unwrap(),
                    ),
                    Res::Def(DefKind::Variant | DefKind::Ctor(CtorOf::Struct, _), def_id) => {
                        tcx.generics_of(tcx.parent(def_id).unwrap())
                    }
                    // Other `DefKind`s don't have generics and would ICE when calling
                    // `generics_of`.
                    Res::Def(
                        DefKind::Struct
                        | DefKind::Union
                        | DefKind::Enum
                        | DefKind::Trait
                        | DefKind::OpaqueTy
                        | DefKind::TyAlias
                        | DefKind::ForeignTy
                        | DefKind::TraitAlias
                        | DefKind::AssocTy
                        | DefKind::Fn
                        | DefKind::AssocFn
                        | DefKind::AssocConst
                        | DefKind::Impl,
                        def_id,
                    ) => tcx.generics_of(def_id),
                    Res::Err => {
                        tcx.sess.delay_span_bug(tcx.def_span(def_id), "anon const with Res::Err");
                        return None;
                    }
                    _ => {
                        // If the user tries to specify generics on a type that does not take them,
                        // e.g. `usize<T>`, we may hit this branch, in which case we treat it as if
                        // no arguments have been passed. An error should already have been emitted.
                        tcx.sess.delay_span_bug(
                            tcx.def_span(def_id),
                            &format!("unexpected anon const res {:?} in path: {:?}", res, path),
                        );
                        return None;
                    }
                };

                generics
                    .params
                    .iter()
                    .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Const { .. }))
                    .nth(arg_index)
                    .map(|param| param.def_id)
            }
            _ => None,
        }
    } else {
        None
    }
}

fn get_path_containing_arg_in_pat<'hir>(
    pat: &'hir hir::Pat<'hir>,
    arg_id: HirId,
) -> Option<&'hir hir::Path<'hir>> {
    use hir::*;

    let is_arg_in_path = |p: &hir::Path<'_>| {
        p.segments
            .iter()
            .filter_map(|seg| seg.args)
            .flat_map(|args| args.args)
            .any(|arg| arg.id() == arg_id)
    };
    let mut arg_path = None;
    pat.walk(|pat| match pat.kind {
        PatKind::Struct(QPath::Resolved(_, path), _, _)
        | PatKind::TupleStruct(QPath::Resolved(_, path), _, _)
        | PatKind::Path(QPath::Resolved(_, path))
            if is_arg_in_path(path) =>
        {
            arg_path = Some(path);
            false
        }
        _ => true,
    });
    arg_path
}

pub(super) fn default_anon_const_substs(tcx: TyCtxt<'_>, def_id: DefId) -> SubstsRef<'_> {
    let generics = tcx.generics_of(def_id);
    if let Some(parent) = generics.parent {
        // This is the reason we bother with having optional anon const substs.
        //
        // In the future the substs of an anon const will depend on its parents predicates
        // at which point eagerly looking at them will cause a query cycle.
        //
        // So for now this is only an assurance that this approach won't cause cycle errors in
        // the future.
        let _cycle_check = tcx.predicates_of(parent);
    }

    let substs = InternalSubsts::identity_for_item(tcx, def_id);
    // We only expect substs with the following type flags as default substs.
    //
    // Getting this wrong can lead to ICE and unsoundness, so we assert it here.
    for arg in substs.iter() {
        let allowed_flags = ty::TypeFlags::MAY_NEED_DEFAULT_CONST_SUBSTS
            | ty::TypeFlags::STILL_FURTHER_SPECIALIZABLE
            | ty::TypeFlags::HAS_ERROR;
        assert!(!arg.has_type_flags(!allowed_flags));
    }
    substs
}

pub(super) fn type_of(tcx: TyCtxt<'_>, def_id: DefId) -> Ty<'_> {
    let def_id = def_id.expect_local();
    use rustc_hir::*;

    let hir_id = tcx.hir().local_def_id_to_hir_id(def_id);

    let icx = ItemCtxt::new(tcx, def_id.to_def_id());

    match tcx.hir().get(hir_id) {
        Node::TraitItem(item) => match item.kind {
            TraitItemKind::Fn(..) => {
                let substs = InternalSubsts::identity_for_item(tcx, def_id.to_def_id());
                tcx.mk_fn_def(def_id.to_def_id(), substs)
            }
            TraitItemKind::Const(ty, body_id) => body_id
                .and_then(|body_id| {
                    if is_suggestable_infer_ty(ty) {
                        Some(infer_placeholder_type(
                            tcx, def_id, body_id, ty.span, item.ident, "constant",
                        ))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| icx.to_ty(ty)),
            TraitItemKind::Type(_, Some(ty)) => icx.to_ty(ty),
            TraitItemKind::Type(_, None) => {
                span_bug!(item.span, "associated type missing default");
            }
        },

        Node::ImplItem(item) => match item.kind {
            ImplItemKind::Fn(..) => {
                let substs = InternalSubsts::identity_for_item(tcx, def_id.to_def_id());
                tcx.mk_fn_def(def_id.to_def_id(), substs)
            }
            ImplItemKind::Const(ty, body_id) => {
                if is_suggestable_infer_ty(ty) {
                    infer_placeholder_type(tcx, def_id, body_id, ty.span, item.ident, "constant")
                } else {
                    icx.to_ty(ty)
                }
            }
            ImplItemKind::TyAlias(ty) => {
                if tcx.impl_trait_ref(tcx.hir().get_parent_did(hir_id).to_def_id()).is_none() {
                    check_feature_inherent_assoc_ty(tcx, item.span);
                }

                icx.to_ty(ty)
            }
        },

        Node::Item(item) => {
            match item.kind {
                ItemKind::Static(ty, .., body_id) => {
                    if is_suggestable_infer_ty(ty) {
                        infer_placeholder_type(
                            tcx,
                            def_id,
                            body_id,
                            ty.span,
                            item.ident,
                            "static variable",
                        )
                    } else {
                        icx.to_ty(ty)
                    }
                }
                ItemKind::Const(ty, body_id) => {
                    if is_suggestable_infer_ty(ty) {
                        infer_placeholder_type(
                            tcx, def_id, body_id, ty.span, item.ident, "constant",
                        )
                    } else {
                        icx.to_ty(ty)
                    }
                }
                ItemKind::TyAlias(self_ty, _)
                | ItemKind::Impl(hir::Impl { self_ty, .. }) => icx.to_ty(self_ty),
                ItemKind::Fn(..) => {
                    let substs = InternalSubsts::identity_for_item(tcx, def_id.to_def_id());
                    tcx.mk_fn_def(def_id.to_def_id(), substs)
                }
                ItemKind::Enum(..) | ItemKind::Struct(..) | ItemKind::Union(..) => {
                    let def = tcx.adt_def(def_id);
                    let substs = InternalSubsts::identity_for_item(tcx, def_id.to_def_id());
                    tcx.mk_adt(def, substs)
                }
                ItemKind::OpaqueTy(OpaqueTy { origin: hir::OpaqueTyOrigin::TyAlias, .. }) => {
                    find_opaque_ty_constraints(tcx, def_id)
                }
                // Opaque types desugared from `impl Trait`.
                ItemKind::OpaqueTy(OpaqueTy { origin: hir::OpaqueTyOrigin::FnReturn(owner) | hir::OpaqueTyOrigin::AsyncFn(owner), .. }) => {
                    let concrete_ty = tcx
                        .mir_borrowck(owner)
                        .concrete_opaque_types
                        .get_value_matching(|(key, _)| key.def_id == def_id.to_def_id())
                        .copied()
                        .unwrap_or_else(|| {
                            tcx.sess.delay_span_bug(
                                DUMMY_SP,
                                &format!(
                                    "owner {:?} has no opaque type for {:?} in its typeck results",
                                    owner, def_id,
                                ),
                            );
                            if let Some(ErrorReported) =
                                tcx.typeck(owner).tainted_by_errors
                            {
                                // Some error in the
                                // owner fn prevented us from populating
                                // the `concrete_opaque_types` table.
                                tcx.ty_error()
                            } else {
                                // We failed to resolve the opaque type or it
                                // resolves to itself. Return the non-revealed
                                // type, which should result in E0720.
                                tcx.mk_opaque(
                                    def_id.to_def_id(),
                                    InternalSubsts::identity_for_item(tcx, def_id.to_def_id()),
                                )
                            }
                        });
                    debug!("concrete_ty = {:?}", concrete_ty);
                    concrete_ty
                }
                ItemKind::Trait(..)
                | ItemKind::TraitAlias(..)
                | ItemKind::Macro(..)
                | ItemKind::Mod(..)
                | ItemKind::ForeignMod { .. }
                | ItemKind::GlobalAsm(..)
                | ItemKind::ExternCrate(..)
                | ItemKind::Use(..) => {
                    span_bug!(
                        item.span,
                        "compute_type_of_item: unexpected item type: {:?}",
                        item.kind
                    );
                }
            }
        }

        Node::ForeignItem(foreign_item) => match foreign_item.kind {
            ForeignItemKind::Fn(..) => {
                let substs = InternalSubsts::identity_for_item(tcx, def_id.to_def_id());
                tcx.mk_fn_def(def_id.to_def_id(), substs)
            }
            ForeignItemKind::Static(t, _) => icx.to_ty(t),
            ForeignItemKind::Type => tcx.mk_foreign(def_id.to_def_id()),
        },

        Node::Ctor(&ref def) | Node::Variant(Variant { data: ref def, .. }) => match *def {
            VariantData::Unit(..) | VariantData::Struct(..) => {
                tcx.type_of(tcx.hir().get_parent_did(hir_id).to_def_id())
            }
            VariantData::Tuple(..) => {
                let substs = InternalSubsts::identity_for_item(tcx, def_id.to_def_id());
                tcx.mk_fn_def(def_id.to_def_id(), substs)
            }
        },

        Node::Field(field) => icx.to_ty(field.ty),

        Node::Expr(&Expr { kind: ExprKind::Closure(..), .. }) => tcx.typeck(def_id).node_type(hir_id),

        Node::AnonConst(_) if let Some(param) = tcx.opt_const_param_of(def_id) => {
            // We defer to `type_of` of the corresponding parameter
            // for generic arguments.
            tcx.type_of(param)
        }

        Node::AnonConst(_) => {
            let parent_node = tcx.hir().get(tcx.hir().get_parent_node(hir_id));
            match parent_node {
                Node::Ty(&Ty { kind: TyKind::Array(_, ref constant), .. })
                | Node::Expr(&Expr { kind: ExprKind::Repeat(_, ref constant), .. })
                    if constant.hir_id() == hir_id =>
                {
                    tcx.types.usize
                }
                Node::Ty(&Ty { kind: TyKind::Typeof(ref e), .. }) if e.hir_id == hir_id => {
                    tcx.typeck(def_id).node_type(e.hir_id)
                }

                Node::Expr(&Expr { kind: ExprKind::ConstBlock(ref anon_const), .. })
                    if anon_const.hir_id == hir_id =>
                {
                    let substs = InternalSubsts::identity_for_item(tcx, def_id.to_def_id());
                    substs.as_inline_const().ty()
                }

                Node::Expr(&Expr { kind: ExprKind::InlineAsm(asm), .. })
                | Node::Item(&Item { kind: ItemKind::GlobalAsm(asm), .. })
                    if asm.operands.iter().any(|(op, _op_sp)| match op {
                        hir::InlineAsmOperand::Const { anon_const } => anon_const.hir_id == hir_id,
                        _ => false,
                    }) =>
                {
                    tcx.typeck(def_id).node_type(hir_id)
                }

                Node::Variant(Variant { disr_expr: Some(ref e), .. }) if e.hir_id == hir_id => tcx
                    .adt_def(tcx.hir().get_parent_did(hir_id).to_def_id())
                    .repr
                    .discr_type()
                    .to_ty(tcx),

                Node::GenericParam(&GenericParam {
                    hir_id: param_hir_id,
                    kind: GenericParamKind::Const { default: Some(ct), .. },
                    ..
                }) if ct.hir_id == hir_id => tcx.type_of(tcx.hir().local_def_id(param_hir_id)),

                x => tcx.ty_error_with_message(
                    DUMMY_SP,
                    &format!("unexpected const parent in type_of(): {:?}", x),
                ),
            }
        }

        Node::GenericParam(param) => match &param.kind {
            GenericParamKind::Type { default: Some(ty), .. }
            | GenericParamKind::Const { ty, .. } => icx.to_ty(ty),
            x => bug!("unexpected non-type Node::GenericParam: {:?}", x),
        },

        x => {
            bug!("unexpected sort of node in type_of(): {:?}", x);
        }
    }
}

#[instrument(skip(tcx), level = "debug")]
/// Checks "defining uses" of opaque `impl Trait` types to ensure that they meet the restrictions
/// laid for "higher-order pattern unification".
/// This ensures that inference is tractable.
/// In particular, definitions of opaque types can only use other generics as arguments,
/// and they cannot repeat an argument. Example:
///
/// ```rust
/// type Foo<A, B> = impl Bar<A, B>;
///
/// // Okay -- `Foo` is applied to two distinct, generic types.
/// fn a<T, U>() -> Foo<T, U> { .. }
///
/// // Not okay -- `Foo` is applied to `T` twice.
/// fn b<T>() -> Foo<T, T> { .. }
///
/// // Not okay -- `Foo` is applied to a non-generic type.
/// fn b<T>() -> Foo<T, u32> { .. }
/// ```
///
fn find_opaque_ty_constraints(tcx: TyCtxt<'_>, def_id: LocalDefId) -> Ty<'_> {
    use rustc_hir::{Expr, ImplItem, Item, TraitItem};

    struct ConstraintLocator<'tcx> {
        tcx: TyCtxt<'tcx>,

        /// def_id of the opaque type whose defining uses are being checked
        def_id: DefId,

        /// as we walk the defining uses, we are checking that all of them
        /// define the same hidden type. This variable is set to `Some`
        /// with the first type that we find, and then later types are
        /// checked against it (we also carry the span of that first
        /// type).
        found: Option<(Span, Ty<'tcx>)>,
    }

    impl ConstraintLocator<'_> {
        #[instrument(skip(self), level = "debug")]
        fn check(&mut self, def_id: LocalDefId) {
            // Don't try to check items that cannot possibly constrain the type.
            if !self.tcx.has_typeck_results(def_id) {
                debug!("no constraint: no typeck results");
                return;
            }
            // Calling `mir_borrowck` can lead to cycle errors through
            // const-checking, avoid calling it if we don't have to.
            if !self.tcx.typeck(def_id).concrete_opaque_types.contains(&self.def_id) {
                debug!("no constraints in typeck results");
                return;
            }
            // Use borrowck to get the type with unerased regions.
            let concrete_opaque_types = &self.tcx.mir_borrowck(def_id).concrete_opaque_types;
            debug!(?concrete_opaque_types);
            for (opaque_type_key, concrete_type) in concrete_opaque_types {
                if opaque_type_key.def_id != self.def_id {
                    // Ignore constraints for other opaque types.
                    continue;
                }

                debug!(?concrete_type, ?opaque_type_key.substs, "found constraint");

                // FIXME(oli-obk): trace the actual span from inference to improve errors.
                let span = self.tcx.def_span(def_id);

                if let Some((prev_span, prev_ty)) = self.found {
                    if *concrete_type != prev_ty && !(*concrete_type, prev_ty).references_error() {
                        debug!(?span);
                        // Found different concrete types for the opaque type.
                        let mut err = self.tcx.sess.struct_span_err(
                            span,
                            "concrete type differs from previous defining opaque type use",
                        );
                        err.span_label(
                            span,
                            format!("expected `{}`, got `{}`", prev_ty, concrete_type),
                        );
                        err.span_note(prev_span, "previous use here");
                        err.emit();
                    }
                } else {
                    self.found = Some((span, concrete_type));
                }
            }
        }
    }

    impl<'tcx> intravisit::Visitor<'tcx> for ConstraintLocator<'tcx> {
        type Map = Map<'tcx>;

        fn nested_visit_map(&mut self) -> intravisit::NestedVisitorMap<Self::Map> {
            intravisit::NestedVisitorMap::All(self.tcx.hir())
        }
        fn visit_expr(&mut self, ex: &'tcx Expr<'tcx>) {
            if let hir::ExprKind::Closure(..) = ex.kind {
                let def_id = self.tcx.hir().local_def_id(ex.hir_id);
                self.check(def_id);
            }
            intravisit::walk_expr(self, ex);
        }
        fn visit_item(&mut self, it: &'tcx Item<'tcx>) {
            debug!("find_existential_constraints: visiting {:?}", it);
            // The opaque type itself or its children are not within its reveal scope.
            if it.def_id.to_def_id() != self.def_id {
                self.check(it.def_id);
                intravisit::walk_item(self, it);
            }
        }
        fn visit_impl_item(&mut self, it: &'tcx ImplItem<'tcx>) {
            debug!("find_existential_constraints: visiting {:?}", it);
            // The opaque type itself or its children are not within its reveal scope.
            if it.def_id.to_def_id() != self.def_id {
                self.check(it.def_id);
                intravisit::walk_impl_item(self, it);
            }
        }
        fn visit_trait_item(&mut self, it: &'tcx TraitItem<'tcx>) {
            debug!("find_existential_constraints: visiting {:?}", it);
            self.check(it.def_id);
            intravisit::walk_trait_item(self, it);
        }
    }

    let hir_id = tcx.hir().local_def_id_to_hir_id(def_id);
    let scope = tcx.hir().get_defining_scope(hir_id);
    let mut locator = ConstraintLocator { def_id: def_id.to_def_id(), tcx, found: None };

    debug!("find_opaque_ty_constraints: scope={:?}", scope);

    if scope == hir::CRATE_HIR_ID {
        tcx.hir().walk_toplevel_module(&mut locator);
    } else {
        debug!("find_opaque_ty_constraints: scope={:?}", tcx.hir().get(scope));
        match tcx.hir().get(scope) {
            // We explicitly call `visit_*` methods, instead of using `intravisit::walk_*` methods
            // This allows our visitor to process the defining item itself, causing
            // it to pick up any 'sibling' defining uses.
            //
            // For example, this code:
            // ```
            // fn foo() {
            //     type Blah = impl Debug;
            //     let my_closure = || -> Blah { true };
            // }
            // ```
            //
            // requires us to explicitly process `foo()` in order
            // to notice the defining usage of `Blah`.
            Node::Item(it) => locator.visit_item(it),
            Node::ImplItem(it) => locator.visit_impl_item(it),
            Node::TraitItem(it) => locator.visit_trait_item(it),
            other => bug!("{:?} is not a valid scope for an opaque type item", other),
        }
    }

    match locator.found {
        Some((_, ty)) => ty,
        None => {
            let span = tcx.def_span(def_id);
            tcx.sess.span_err(span, "could not find defining uses");
            tcx.ty_error()
        }
    }
}

fn infer_placeholder_type<'a>(
    tcx: TyCtxt<'a>,
    def_id: LocalDefId,
    body_id: hir::BodyId,
    span: Span,
    item_ident: Ident,
    kind: &'static str,
) -> Ty<'a> {
    // Attempts to make the type nameable by turning FnDefs into FnPtrs.
    struct MakeNameable<'tcx> {
        success: bool,
        tcx: TyCtxt<'tcx>,
    }

    impl<'tcx> MakeNameable<'tcx> {
        fn new(tcx: TyCtxt<'tcx>) -> Self {
            MakeNameable { success: true, tcx }
        }
    }

    impl<'tcx> TypeFolder<'tcx> for MakeNameable<'tcx> {
        fn tcx(&self) -> TyCtxt<'tcx> {
            self.tcx
        }

        fn fold_ty(&mut self, ty: Ty<'tcx>) -> Ty<'tcx> {
            if !self.success {
                return ty;
            }

            match ty.kind() {
                ty::FnDef(def_id, _) => self.tcx.mk_fn_ptr(self.tcx.fn_sig(*def_id)),
                // FIXME: non-capturing closures should also suggest a function pointer
                ty::Closure(..) | ty::Generator(..) => {
                    self.success = false;
                    ty
                }
                _ => ty.super_fold_with(self),
            }
        }
    }

    let ty = tcx.diagnostic_only_typeck(def_id).node_type(body_id.hir_id);

    // If this came from a free `const` or `static mut?` item,
    // then the user may have written e.g. `const A = 42;`.
    // In this case, the parser has stashed a diagnostic for
    // us to improve in typeck so we do that now.
    match tcx.sess.diagnostic().steal_diagnostic(span, StashKey::ItemNoType) {
        Some(mut err) => {
            if !ty.references_error() {
                // The parser provided a sub-optimal `HasPlaceholders` suggestion for the type.
                // We are typeck and have the real type, so remove that and suggest the actual type.
                err.suggestions.clear();

                // Suggesting unnameable types won't help.
                let mut mk_nameable = MakeNameable::new(tcx);
                let ty = mk_nameable.fold_ty(ty);
                let sugg_ty = if mk_nameable.success { Some(ty) } else { None };
                if let Some(sugg_ty) = sugg_ty {
                    err.span_suggestion(
                        span,
                        &format!("provide a type for the {item}", item = kind),
                        format!("{}: {}", item_ident, sugg_ty),
                        Applicability::MachineApplicable,
                    );
                } else {
                    err.span_note(
                        tcx.hir().body(body_id).value.span,
                        &format!("however, the inferred type `{}` cannot be named", ty),
                    );
                }
            }

            err.emit();
        }
        None => {
            let mut diag = bad_placeholder(tcx, "type", vec![span], kind);

            if !ty.references_error() {
                let mut mk_nameable = MakeNameable::new(tcx);
                let ty = mk_nameable.fold_ty(ty);
                let sugg_ty = if mk_nameable.success { Some(ty) } else { None };
                if let Some(sugg_ty) = sugg_ty {
                    diag.span_suggestion(
                        span,
                        "replace with the correct type",
                        sugg_ty.to_string(),
                        Applicability::MaybeIncorrect,
                    );
                } else {
                    diag.span_note(
                        tcx.hir().body(body_id).value.span,
                        &format!("however, the inferred type `{}` cannot be named", ty),
                    );
                }
            }

            diag.emit();
        }
    }

    // Typeck doesn't expect erased regions to be returned from `type_of`.
    tcx.fold_regions(ty, &mut false, |r, _| match r {
        ty::ReErased => tcx.lifetimes.re_static,
        _ => r,
    })
}

fn check_feature_inherent_assoc_ty(tcx: TyCtxt<'_>, span: Span) {
    if !tcx.features().inherent_associated_types {
        use rustc_session::parse::feature_err;
        use rustc_span::symbol::sym;
        feature_err(
            &tcx.sess.parse_sess,
            sym::inherent_associated_types,
            span,
            "inherent associated types are unstable",
        )
        .emit();
    }
}
