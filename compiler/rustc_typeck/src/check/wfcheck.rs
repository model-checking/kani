use crate::check::regionck::OutlivesEnvironmentExt;
use crate::check::{FnCtxt, Inherited};
use crate::constrained_generic_params::{identify_constrained_generic_params, Parameter};

use rustc_ast as ast;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::{struct_span_err, Applicability, DiagnosticBuilder};
use rustc_hir as hir;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit as hir_visit;
use rustc_hir::intravisit::Visitor;
use rustc_hir::itemlikevisit::ParItemLikeVisitor;
use rustc_hir::lang_items::LangItem;
use rustc_hir::ItemKind;
use rustc_infer::infer::outlives::env::OutlivesEnvironment;
use rustc_infer::infer::outlives::obligations::TypeOutlives;
use rustc_infer::infer::TyCtxtInferExt;
use rustc_infer::infer::{self, RegionckMode, SubregionOrigin};
use rustc_middle::hir::map as hir_map;
use rustc_middle::ty::subst::{GenericArgKind, InternalSubsts, Subst};
use rustc_middle::ty::trait_def::TraitSpecializationKind;
use rustc_middle::ty::{
    self, AdtKind, GenericParamDefKind, ToPredicate, Ty, TyCtxt, TypeFoldable, TypeVisitor,
};
use rustc_session::parse::feature_err;
use rustc_span::symbol::{sym, Ident, Symbol};
use rustc_span::{Span, DUMMY_SP};
use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt as _;
use rustc_trait_selection::traits::{self, ObligationCause, ObligationCauseCode, WellFormedLoc};

use std::convert::TryInto;
use std::iter;
use std::ops::ControlFlow;

/// Helper type of a temporary returned by `.for_item(...)`.
/// This is necessary because we can't write the following bound:
///
/// ```rust
/// F: for<'b, 'tcx> where 'tcx FnOnce(FnCtxt<'b, 'tcx>)
/// ```
struct CheckWfFcxBuilder<'tcx> {
    inherited: super::InheritedBuilder<'tcx>,
    id: hir::HirId,
    span: Span,
    param_env: ty::ParamEnv<'tcx>,
}

impl<'tcx> CheckWfFcxBuilder<'tcx> {
    fn with_fcx<F>(&mut self, f: F)
    where
        F: for<'b> FnOnce(&FnCtxt<'b, 'tcx>) -> FxHashSet<Ty<'tcx>>,
    {
        let id = self.id;
        let span = self.span;
        let param_env = self.param_env;
        self.inherited.enter(|inh| {
            let fcx = FnCtxt::new(&inh, param_env, id);
            if !inh.tcx.features().trivial_bounds {
                // As predicates are cached rather than obligations, this
                // needs to be called first so that they are checked with an
                // empty `param_env`.
                check_false_global_bounds(&fcx, span, id);
            }
            let wf_tys = f(&fcx);
            fcx.select_all_obligations_or_error();
            fcx.regionck_item(id, span, wf_tys);
        });
    }
}

/// Checks that the field types (in a struct def'n) or argument types (in an enum def'n) are
/// well-formed, meaning that they do not require any constraints not declared in the struct
/// definition itself. For example, this definition would be illegal:
///
/// ```rust
/// struct Ref<'a, T> { x: &'a T }
/// ```
///
/// because the type did not declare that `T:'a`.
///
/// We do this check as a pre-pass before checking fn bodies because if these constraints are
/// not included it frequently leads to confusing errors in fn bodies. So it's better to check
/// the types first.
#[instrument(skip(tcx), level = "debug")]
pub fn check_item_well_formed(tcx: TyCtxt<'_>, def_id: LocalDefId) {
    let item = tcx.hir().expect_item(def_id);

    debug!(
        ?item.def_id,
        item.name = ? tcx.def_path_str(def_id.to_def_id())
    );

    match item.kind {
        // Right now we check that every default trait implementation
        // has an implementation of itself. Basically, a case like:
        //
        //     impl Trait for T {}
        //
        // has a requirement of `T: Trait` which was required for default
        // method implementations. Although this could be improved now that
        // there's a better infrastructure in place for this, it's being left
        // for a follow-up work.
        //
        // Since there's such a requirement, we need to check *just* positive
        // implementations, otherwise things like:
        //
        //     impl !Send for T {}
        //
        // won't be allowed unless there's an *explicit* implementation of `Send`
        // for `T`
        hir::ItemKind::Impl(ref impl_) => {
            let is_auto = tcx
                .impl_trait_ref(item.def_id)
                .map_or(false, |trait_ref| tcx.trait_is_auto(trait_ref.def_id));
            if let (hir::Defaultness::Default { .. }, true) = (impl_.defaultness, is_auto) {
                let sp = impl_.of_trait.as_ref().map_or(item.span, |t| t.path.span);
                let mut err =
                    tcx.sess.struct_span_err(sp, "impls of auto traits cannot be default");
                err.span_labels(impl_.defaultness_span, "default because of this");
                err.span_label(sp, "auto trait");
                err.emit();
            }
            // We match on both `ty::ImplPolarity` and `ast::ImplPolarity` just to get the `!` span.
            match (tcx.impl_polarity(def_id), impl_.polarity) {
                (ty::ImplPolarity::Positive, _) => {
                    check_impl(tcx, item, impl_.self_ty, &impl_.of_trait);
                }
                (ty::ImplPolarity::Negative, ast::ImplPolarity::Negative(span)) => {
                    // FIXME(#27579): what amount of WF checking do we need for neg impls?
                    if let hir::Defaultness::Default { .. } = impl_.defaultness {
                        let mut spans = vec![span];
                        spans.extend(impl_.defaultness_span);
                        struct_span_err!(
                            tcx.sess,
                            spans,
                            E0750,
                            "negative impls cannot be default impls"
                        )
                        .emit();
                    }
                }
                (ty::ImplPolarity::Reservation, _) => {
                    // FIXME: what amount of WF checking do we need for reservation impls?
                }
                _ => unreachable!(),
            }
        }
        hir::ItemKind::Fn(ref sig, ..) => {
            check_item_fn(tcx, item.def_id, item.ident, item.span, sig.decl);
        }
        hir::ItemKind::Static(ty, ..) => {
            check_item_type(tcx, item.def_id, ty.span, false);
        }
        hir::ItemKind::Const(ty, ..) => {
            check_item_type(tcx, item.def_id, ty.span, false);
        }
        hir::ItemKind::ForeignMod { items, .. } => {
            for it in items.iter() {
                let it = tcx.hir().foreign_item(it.id);
                match it.kind {
                    hir::ForeignItemKind::Fn(decl, ..) => {
                        check_item_fn(tcx, it.def_id, it.ident, it.span, decl)
                    }
                    hir::ForeignItemKind::Static(ty, ..) => {
                        check_item_type(tcx, it.def_id, ty.span, true)
                    }
                    hir::ForeignItemKind::Type => (),
                }
            }
        }
        hir::ItemKind::Struct(ref struct_def, ref ast_generics) => {
            check_type_defn(tcx, item, false, |fcx| vec![fcx.non_enum_variant(struct_def)]);

            check_variances_for_type_defn(tcx, item, ast_generics);
        }
        hir::ItemKind::Union(ref struct_def, ref ast_generics) => {
            check_type_defn(tcx, item, true, |fcx| vec![fcx.non_enum_variant(struct_def)]);

            check_variances_for_type_defn(tcx, item, ast_generics);
        }
        hir::ItemKind::Enum(ref enum_def, ref ast_generics) => {
            check_type_defn(tcx, item, true, |fcx| fcx.enum_variants(enum_def));

            check_variances_for_type_defn(tcx, item, ast_generics);
        }
        hir::ItemKind::Trait(..) => {
            check_trait(tcx, item);
        }
        hir::ItemKind::TraitAlias(..) => {
            check_trait(tcx, item);
        }
        _ => {}
    }
}

pub fn check_trait_item(tcx: TyCtxt<'_>, def_id: LocalDefId) {
    let hir_id = tcx.hir().local_def_id_to_hir_id(def_id);
    let trait_item = tcx.hir().expect_trait_item(def_id);

    let (method_sig, span) = match trait_item.kind {
        hir::TraitItemKind::Fn(ref sig, _) => (Some(sig), trait_item.span),
        hir::TraitItemKind::Type(_bounds, Some(ty)) => (None, ty.span),
        _ => (None, trait_item.span),
    };
    check_object_unsafe_self_trait_by_name(tcx, trait_item);
    check_associated_item(tcx, trait_item.def_id, span, method_sig);

    let encl_trait_def_id = tcx.hir().get_parent_did(hir_id);
    let encl_trait = tcx.hir().expect_item(encl_trait_def_id);
    let encl_trait_def_id = encl_trait.def_id.to_def_id();
    let fn_lang_item_name = if Some(encl_trait_def_id) == tcx.lang_items().fn_trait() {
        Some("fn")
    } else if Some(encl_trait_def_id) == tcx.lang_items().fn_mut_trait() {
        Some("fn_mut")
    } else {
        None
    };

    if let (Some(fn_lang_item_name), "call") =
        (fn_lang_item_name, trait_item.ident.name.to_ident_string().as_str())
    {
        // We are looking at the `call` function of the `fn` or `fn_mut` lang item.
        // Do some rudimentary sanity checking to avoid an ICE later (issue #83471).
        if let Some(hir::FnSig { decl, span, .. }) = method_sig {
            if let [self_ty, _] = decl.inputs {
                if !matches!(self_ty.kind, hir::TyKind::Rptr(_, _)) {
                    tcx.sess
                        .struct_span_err(
                            self_ty.span,
                            &format!(
                                "first argument of `call` in `{}` lang item must be a reference",
                                fn_lang_item_name
                            ),
                        )
                        .emit();
                }
            } else {
                tcx.sess
                    .struct_span_err(
                        *span,
                        &format!(
                            "`call` function in `{}` lang item takes exactly two arguments",
                            fn_lang_item_name
                        ),
                    )
                    .emit();
            }
        } else {
            tcx.sess
                .struct_span_err(
                    trait_item.span,
                    &format!(
                        "`call` trait item in `{}` lang item must be a function",
                        fn_lang_item_name
                    ),
                )
                .emit();
        }
    }

    check_gat_where_clauses(tcx, trait_item, encl_trait_def_id);
}

/// Require that the user writes where clauses on GATs for the implicit
/// outlives bounds involving trait parameters in trait functions and
/// lifetimes passed as GAT substs. See `self-outlives-lint` test.
///
/// This trait will be our running example. We are currently WF checking the `Item` item...
///
/// ```rust
/// trait LendingIterator {
///   type Item<'me>; // <-- WF checking this trait item
///
///   fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
/// }
/// ```
fn check_gat_where_clauses(
    tcx: TyCtxt<'_>,
    trait_item: &hir::TraitItem<'_>,
    encl_trait_def_id: DefId,
) {
    let item = tcx.associated_item(trait_item.def_id);
    // If the current trait item isn't a type, it isn't a GAT
    if !matches!(item.kind, ty::AssocKind::Type) {
        return;
    }
    let generics: &ty::Generics = tcx.generics_of(trait_item.def_id);
    // If the current associated type doesn't have any (own) params, it's not a GAT
    // FIXME(jackh726): we can also warn in the more general case
    if generics.params.len() == 0 {
        return;
    }
    let associated_items: &ty::AssocItems<'_> = tcx.associated_items(encl_trait_def_id);
    let mut clauses: Option<FxHashSet<ty::Predicate<'_>>> = None;
    // For every function in this trait...
    // In our example, this would be the `next` method
    for item in
        associated_items.in_definition_order().filter(|item| matches!(item.kind, ty::AssocKind::Fn))
    {
        // The clauses we that we would require from this function
        let mut function_clauses = FxHashSet::default();

        let id = hir::HirId::make_owner(item.def_id.expect_local());
        let param_env = tcx.param_env(item.def_id.expect_local());

        let sig = tcx.fn_sig(item.def_id);
        // Get the signature using placeholders. In our example, this would
        // convert the late-bound 'a into a free region.
        let sig = tcx.liberate_late_bound_regions(item.def_id, sig);
        // Collect the arguments that are given to this GAT in the return type
        // of  the function signature. In our example, the GAT in the return
        // type is `<Self as LendingIterator>::Item<'a>`, so 'a and Self are arguments.
        let (regions, types) =
            GATSubstCollector::visit(tcx, trait_item.def_id.to_def_id(), sig.output());

        // If both regions and types are empty, then this GAT isn't in the
        // return type, and we shouldn't try to do clause analysis
        // (particularly, doing so would end up with an empty set of clauses,
        // since the current method would require none, and we take the
        // intersection of requirements of all methods)
        if types.is_empty() && regions.is_empty() {
            continue;
        }

        // The types we can assume to be well-formed. In our example, this
        // would be &'a mut Self, from the first argument.
        let mut wf_tys = FxHashSet::default();
        wf_tys.extend(sig.inputs());

        // For each region argument (e.g., 'a in our example), check for a
        // relationship to the type arguments (e.g., Self). If there is an
        // outlives relationship (`Self: 'a`), then we want to ensure that is
        // reflected in a where clause on the GAT itself.
        for (region, region_idx) in &regions {
            for (ty, ty_idx) in &types {
                // In our example, requires that Self: 'a
                if ty_known_to_outlive(tcx, id, param_env, &wf_tys, *ty, *region) {
                    debug!(?ty_idx, ?region_idx);
                    debug!("required clause: {} must outlive {}", ty, region);
                    // Translate into the generic parameters of the GAT. In
                    // our example, the type was Self, which will also be
                    // Self in the GAT.
                    let ty_param = generics.param_at(*ty_idx, tcx);
                    let ty_param = tcx.mk_ty(ty::Param(ty::ParamTy {
                        index: ty_param.index,
                        name: ty_param.name,
                    }));
                    // Same for the region. In our example, 'a corresponds
                    // to the 'me parameter.
                    let region_param = generics.param_at(*region_idx, tcx);
                    let region_param =
                        tcx.mk_region(ty::RegionKind::ReEarlyBound(ty::EarlyBoundRegion {
                            def_id: region_param.def_id,
                            index: region_param.index,
                            name: region_param.name,
                        }));
                    // The predicate we expect to see. (In our example,
                    // `Self: 'me`.)
                    let clause = ty::PredicateKind::TypeOutlives(ty::OutlivesPredicate(
                        ty_param,
                        region_param,
                    ));
                    let clause = tcx.mk_predicate(ty::Binder::dummy(clause));
                    function_clauses.insert(clause);
                }
            }
        }

        // For each region argument (e.g., 'a in our example), also check for a
        // relationship to the other region arguments. If there is an
        // outlives relationship, then we want to ensure that is
        // reflected in a where clause on the GAT itself.
        for (region_a, region_a_idx) in &regions {
            for (region_b, region_b_idx) in &regions {
                if region_a == region_b {
                    continue;
                }

                if region_known_to_outlive(tcx, id, param_env, &wf_tys, *region_a, *region_b) {
                    debug!(?region_a_idx, ?region_b_idx);
                    debug!("required clause: {} must outlive {}", region_a, region_b);
                    // Translate into the generic parameters of the GAT.
                    let region_a_param = generics.param_at(*region_a_idx, tcx);
                    let region_a_param =
                        tcx.mk_region(ty::RegionKind::ReEarlyBound(ty::EarlyBoundRegion {
                            def_id: region_a_param.def_id,
                            index: region_a_param.index,
                            name: region_a_param.name,
                        }));
                    // Same for the region.
                    let region_b_param = generics.param_at(*region_b_idx, tcx);
                    let region_b_param =
                        tcx.mk_region(ty::RegionKind::ReEarlyBound(ty::EarlyBoundRegion {
                            def_id: region_b_param.def_id,
                            index: region_b_param.index,
                            name: region_b_param.name,
                        }));
                    // The predicate we expect to see.
                    let clause = ty::PredicateKind::RegionOutlives(ty::OutlivesPredicate(
                        region_a_param,
                        region_b_param,
                    ));
                    let clause = tcx.mk_predicate(ty::Binder::dummy(clause));
                    function_clauses.insert(clause);
                }
            }
        }

        // Imagine we have:
        // ```
        // trait Foo {
        //   type Bar<'me>;
        //   fn gimme(&self) -> Self::Bar<'_>;
        //   fn gimme_default(&self) -> Self::Bar<'static>;
        // }
        // ```
        // We only want to require clauses on `Bar` that we can prove from *all* functions (in this
        // case, `'me` can be `static` from `gimme_default`)
        match clauses.as_mut() {
            Some(clauses) => {
                clauses.drain_filter(|p| !function_clauses.contains(p));
            }
            None => {
                clauses = Some(function_clauses);
            }
        }
    }

    // If there are any clauses that aren't provable, emit an error
    let clauses = clauses.unwrap_or_default();
    debug!(?clauses);
    if !clauses.is_empty() {
        let param_env = tcx.param_env(trait_item.def_id);

        let mut clauses: Vec<_> = clauses
            .into_iter()
            .filter(|clause| match clause.kind().skip_binder() {
                ty::PredicateKind::RegionOutlives(ty::OutlivesPredicate(a, b)) => {
                    !region_known_to_outlive(
                        tcx,
                        trait_item.hir_id(),
                        param_env,
                        &FxHashSet::default(),
                        a,
                        b,
                    )
                }
                ty::PredicateKind::TypeOutlives(ty::OutlivesPredicate(a, b)) => {
                    !ty_known_to_outlive(
                        tcx,
                        trait_item.hir_id(),
                        param_env,
                        &FxHashSet::default(),
                        a,
                        b,
                    )
                }
                _ => bug!("Unexpected PredicateKind"),
            })
            .map(|clause| format!("{}", clause))
            .collect();

        // We sort so that order is predictable
        clauses.sort();

        if !clauses.is_empty() {
            let plural = if clauses.len() > 1 { "s" } else { "" };
            let mut err = tcx.sess.struct_span_err(
                trait_item.span,
                &format!("missing required bound{} on `{}`", plural, trait_item.ident),
            );

            let suggestion = format!(
                "{} {}",
                if !trait_item.generics.where_clause.predicates.is_empty() {
                    ","
                } else {
                    " where"
                },
                clauses.join(", "),
            );
            err.span_suggestion(
                trait_item.generics.where_clause.tail_span_for_suggestion(),
                &format!("add the required where clause{}", plural),
                suggestion,
                Applicability::MachineApplicable,
            );

            let bound = if clauses.len() > 1 { "these bounds are" } else { "this bound is" };
            err.note(&format!(
                "{} currently required to ensure that impls have maximum flexibility",
                bound
            ));
            err.note(
                "we are soliciting feedback, see issue #87479 \
                 <https://github.com/rust-lang/rust/issues/87479> \
                 for more information",
            );

            err.emit()
        }
    }
}

// FIXME(jackh726): refactor some of the shared logic between the two functions below

/// Given a known `param_env` and a set of well formed types, can we prove that
/// `ty` outlives `region`.
fn ty_known_to_outlive<'tcx>(
    tcx: TyCtxt<'tcx>,
    id: hir::HirId,
    param_env: ty::ParamEnv<'tcx>,
    wf_tys: &FxHashSet<Ty<'tcx>>,
    ty: Ty<'tcx>,
    region: ty::Region<'tcx>,
) -> bool {
    // Unfortunately, we have to use a new `InferCtxt` each call, because
    // region constraints get added and solved there and we need to test each
    // call individually.
    tcx.infer_ctxt().enter(|infcx| {
        let mut outlives_environment = OutlivesEnvironment::new(param_env);
        outlives_environment.add_implied_bounds(&infcx, wf_tys.clone(), id, DUMMY_SP);
        outlives_environment.save_implied_bounds(id);
        let region_bound_pairs = outlives_environment.region_bound_pairs_map().get(&id).unwrap();

        let cause = ObligationCause::new(DUMMY_SP, id, ObligationCauseCode::MiscObligation);

        let sup_type = ty;
        let sub_region = region;

        let origin = SubregionOrigin::from_obligation_cause(&cause, || {
            infer::RelateParamBound(cause.span, sup_type, None)
        });

        let outlives = &mut TypeOutlives::new(
            &infcx,
            tcx,
            &region_bound_pairs,
            Some(infcx.tcx.lifetimes.re_root_empty),
            param_env,
        );
        outlives.type_must_outlive(origin, sup_type, sub_region);

        let errors = infcx.resolve_regions(
            id.expect_owner().to_def_id(),
            &outlives_environment,
            RegionckMode::default(),
        );

        debug!(?errors, "errors");

        // If we were able to prove that the type outlives the region without
        // an error, it must be because of the implied or explicit bounds...
        errors.is_empty()
    })
}

fn region_known_to_outlive<'tcx>(
    tcx: TyCtxt<'tcx>,
    id: hir::HirId,
    param_env: ty::ParamEnv<'tcx>,
    wf_tys: &FxHashSet<Ty<'tcx>>,
    region_a: ty::Region<'tcx>,
    region_b: ty::Region<'tcx>,
) -> bool {
    // Unfortunately, we have to use a new `InferCtxt` each call, because
    // region constraints get added and solved there and we need to test each
    // call individually.
    tcx.infer_ctxt().enter(|infcx| {
        let mut outlives_environment = OutlivesEnvironment::new(param_env);
        outlives_environment.add_implied_bounds(&infcx, wf_tys.clone(), id, DUMMY_SP);
        outlives_environment.save_implied_bounds(id);

        let cause = ObligationCause::new(DUMMY_SP, id, ObligationCauseCode::MiscObligation);

        let origin = SubregionOrigin::from_obligation_cause(&cause, || {
            infer::RelateRegionParamBound(cause.span)
        });

        use rustc_infer::infer::outlives::obligations::TypeOutlivesDelegate;
        // `region_a: region_b` -> `region_b <= region_a`
        (&infcx).push_sub_region_constraint(origin, region_b, region_a);

        let errors = infcx.resolve_regions(
            id.expect_owner().to_def_id(),
            &outlives_environment,
            RegionckMode::default(),
        );

        debug!(?errors, "errors");

        // If we were able to prove that the type outlives the region without
        // an error, it must be because of the implied or explicit bounds...
        errors.is_empty()
    })
}

/// TypeVisitor that looks for uses of GATs like
/// `<P0 as Trait<P1..Pn>>::GAT<Pn..Pm>` and adds the arguments `P0..Pm` into
/// the two vectors, `regions` and `types` (depending on their kind). For each
/// parameter `Pi` also track the index `i`.
struct GATSubstCollector<'tcx> {
    tcx: TyCtxt<'tcx>,
    gat: DefId,
    // Which region appears and which parameter index its subsituted for
    regions: FxHashSet<(ty::Region<'tcx>, usize)>,
    // Which params appears and which parameter index its subsituted for
    types: FxHashSet<(Ty<'tcx>, usize)>,
}

impl<'tcx> GATSubstCollector<'tcx> {
    fn visit<T: TypeFoldable<'tcx>>(
        tcx: TyCtxt<'tcx>,
        gat: DefId,
        t: T,
    ) -> (FxHashSet<(ty::Region<'tcx>, usize)>, FxHashSet<(Ty<'tcx>, usize)>) {
        let mut visitor = GATSubstCollector {
            tcx,
            gat,
            regions: FxHashSet::default(),
            types: FxHashSet::default(),
        };
        t.visit_with(&mut visitor);
        (visitor.regions, visitor.types)
    }
}

impl<'tcx> TypeVisitor<'tcx> for GATSubstCollector<'tcx> {
    type BreakTy = !;

    fn visit_ty(&mut self, t: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {
        match t.kind() {
            ty::Projection(p) if p.item_def_id == self.gat => {
                for (idx, subst) in p.substs.iter().enumerate() {
                    match subst.unpack() {
                        GenericArgKind::Lifetime(lt) => {
                            self.regions.insert((lt, idx));
                        }
                        GenericArgKind::Type(t) => {
                            self.types.insert((t, idx));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        t.super_visit_with(self)
    }

    fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
        Some(self.tcx)
    }
}

fn could_be_self(trait_def_id: LocalDefId, ty: &hir::Ty<'_>) -> bool {
    match ty.kind {
        hir::TyKind::TraitObject([trait_ref], ..) => match trait_ref.trait_ref.path.segments {
            [s] => s.res.and_then(|r| r.opt_def_id()) == Some(trait_def_id.to_def_id()),
            _ => false,
        },
        _ => false,
    }
}

/// Detect when an object unsafe trait is referring to itself in one of its associated items.
/// When this is done, suggest using `Self` instead.
fn check_object_unsafe_self_trait_by_name(tcx: TyCtxt<'_>, item: &hir::TraitItem<'_>) {
    let (trait_name, trait_def_id) = match tcx.hir().get(tcx.hir().get_parent_item(item.hir_id())) {
        hir::Node::Item(item) => match item.kind {
            hir::ItemKind::Trait(..) => (item.ident, item.def_id),
            _ => return,
        },
        _ => return,
    };
    let mut trait_should_be_self = vec![];
    match &item.kind {
        hir::TraitItemKind::Const(ty, _) | hir::TraitItemKind::Type(_, Some(ty))
            if could_be_self(trait_def_id, ty) =>
        {
            trait_should_be_self.push(ty.span)
        }
        hir::TraitItemKind::Fn(sig, _) => {
            for ty in sig.decl.inputs {
                if could_be_self(trait_def_id, ty) {
                    trait_should_be_self.push(ty.span);
                }
            }
            match sig.decl.output {
                hir::FnRetTy::Return(ty) if could_be_self(trait_def_id, ty) => {
                    trait_should_be_self.push(ty.span);
                }
                _ => {}
            }
        }
        _ => {}
    }
    if !trait_should_be_self.is_empty() {
        if tcx.object_safety_violations(trait_def_id).is_empty() {
            return;
        }
        let sugg = trait_should_be_self.iter().map(|span| (*span, "Self".to_string())).collect();
        tcx.sess
            .struct_span_err(
                trait_should_be_self,
                "associated item referring to unboxed trait object for its own trait",
            )
            .span_label(trait_name.span, "in this trait")
            .multipart_suggestion(
                "you might have meant to use `Self` to refer to the implementing type",
                sugg,
                Applicability::MachineApplicable,
            )
            .emit();
    }
}

pub fn check_impl_item(tcx: TyCtxt<'_>, def_id: LocalDefId) {
    let impl_item = tcx.hir().expect_impl_item(def_id);

    let (method_sig, span) = match impl_item.kind {
        hir::ImplItemKind::Fn(ref sig, _) => (Some(sig), impl_item.span),
        // Constrain binding and overflow error spans to `<Ty>` in `type foo = <Ty>`.
        hir::ImplItemKind::TyAlias(ty) if ty.span != DUMMY_SP => (None, ty.span),
        _ => (None, impl_item.span),
    };

    check_associated_item(tcx, impl_item.def_id, span, method_sig);
}

fn check_param_wf(tcx: TyCtxt<'_>, param: &hir::GenericParam<'_>) {
    match param.kind {
        // We currently only check wf of const params here.
        hir::GenericParamKind::Lifetime { .. } | hir::GenericParamKind::Type { .. } => (),

        // Const parameters are well formed if their type is structural match.
        hir::GenericParamKind::Const { ty: hir_ty, default: _ } => {
            let ty = tcx.type_of(tcx.hir().local_def_id(param.hir_id));

            let err_ty_str;
            let mut is_ptr = true;
            let err = if tcx.features().adt_const_params {
                match ty.peel_refs().kind() {
                    ty::FnPtr(_) => Some("function pointers"),
                    ty::RawPtr(_) => Some("raw pointers"),
                    _ => None,
                }
            } else {
                match ty.kind() {
                    ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Error(_) => None,
                    ty::FnPtr(_) => Some("function pointers"),
                    ty::RawPtr(_) => Some("raw pointers"),
                    _ => {
                        is_ptr = false;
                        err_ty_str = format!("`{}`", ty);
                        Some(err_ty_str.as_str())
                    }
                }
            };
            if let Some(unsupported_type) = err {
                if is_ptr {
                    tcx.sess.span_err(
                        hir_ty.span,
                        &format!(
                            "using {} as const generic parameters is forbidden",
                            unsupported_type
                        ),
                    )
                } else {
                    let mut err = tcx.sess.struct_span_err(
                        hir_ty.span,
                        &format!(
                            "{} is forbidden as the type of a const generic parameter",
                            unsupported_type
                        ),
                    );
                    err.note("the only supported types are integers, `bool` and `char`");
                    if tcx.sess.is_nightly_build() {
                        err.help(
                            "more complex types are supported with `#![feature(adt_const_params)]`",
                        );
                    }
                    err.emit()
                }
            };

            if traits::search_for_structural_match_violation(param.hir_id, param.span, tcx, ty)
                .is_some()
            {
                // We use the same error code in both branches, because this is really the same
                // issue: we just special-case the message for type parameters to make it
                // clearer.
                if let ty::Param(_) = ty.peel_refs().kind() {
                    // Const parameters may not have type parameters as their types,
                    // because we cannot be sure that the type parameter derives `PartialEq`
                    // and `Eq` (just implementing them is not enough for `structural_match`).
                    struct_span_err!(
                        tcx.sess,
                        hir_ty.span,
                        E0741,
                        "`{}` is not guaranteed to `#[derive(PartialEq, Eq)]`, so may not be \
                            used as the type of a const parameter",
                        ty,
                    )
                    .span_label(
                        hir_ty.span,
                        format!("`{}` may not derive both `PartialEq` and `Eq`", ty),
                    )
                    .note(
                        "it is not currently possible to use a type parameter as the type of a \
                            const parameter",
                    )
                    .emit();
                } else {
                    struct_span_err!(
                        tcx.sess,
                        hir_ty.span,
                        E0741,
                        "`{}` must be annotated with `#[derive(PartialEq, Eq)]` to be used as \
                            the type of a const parameter",
                        ty,
                    )
                    .span_label(
                        hir_ty.span,
                        format!("`{}` doesn't derive both `PartialEq` and `Eq`", ty),
                    )
                    .emit();
                }
            }
        }
    }
}

#[tracing::instrument(level = "debug", skip(tcx, span, sig_if_method))]
fn check_associated_item(
    tcx: TyCtxt<'_>,
    item_id: LocalDefId,
    span: Span,
    sig_if_method: Option<&hir::FnSig<'_>>,
) {
    let code = ObligationCauseCode::WellFormed(Some(WellFormedLoc::Ty(item_id)));
    for_id(tcx, item_id, span).with_fcx(|fcx| {
        let item = fcx.tcx.associated_item(item_id);

        let (mut implied_bounds, self_ty) = match item.container {
            ty::TraitContainer(_) => (FxHashSet::default(), fcx.tcx.types.self_param),
            ty::ImplContainer(def_id) => {
                (fcx.impl_implied_bounds(def_id, span), fcx.tcx.type_of(def_id))
            }
        };

        match item.kind {
            ty::AssocKind::Const => {
                let ty = fcx.tcx.type_of(item.def_id);
                let ty = fcx.normalize_associated_types_in_wf(span, ty, WellFormedLoc::Ty(item_id));
                fcx.register_wf_obligation(ty.into(), span, code.clone());
            }
            ty::AssocKind::Fn => {
                let sig = fcx.tcx.fn_sig(item.def_id);
                let hir_sig = sig_if_method.expect("bad signature for method");
                check_fn_or_method(
                    fcx,
                    item.ident.span,
                    sig,
                    hir_sig.decl,
                    item.def_id,
                    &mut implied_bounds,
                );
                check_method_receiver(fcx, hir_sig, item, self_ty);
            }
            ty::AssocKind::Type => {
                if let ty::AssocItemContainer::TraitContainer(_) = item.container {
                    check_associated_type_bounds(fcx, item, span)
                }
                if item.defaultness.has_value() {
                    let ty = fcx.tcx.type_of(item.def_id);
                    let ty =
                        fcx.normalize_associated_types_in_wf(span, ty, WellFormedLoc::Ty(item_id));
                    fcx.register_wf_obligation(ty.into(), span, code.clone());
                }
            }
        }

        implied_bounds
    })
}

fn for_item<'tcx>(tcx: TyCtxt<'tcx>, item: &hir::Item<'_>) -> CheckWfFcxBuilder<'tcx> {
    for_id(tcx, item.def_id, item.span)
}

fn for_id(tcx: TyCtxt<'_>, def_id: LocalDefId, span: Span) -> CheckWfFcxBuilder<'_> {
    CheckWfFcxBuilder {
        inherited: Inherited::build(tcx, def_id),
        id: hir::HirId::make_owner(def_id),
        span,
        param_env: tcx.param_env(def_id),
    }
}

fn item_adt_kind(kind: &ItemKind<'_>) -> Option<AdtKind> {
    match kind {
        ItemKind::Struct(..) => Some(AdtKind::Struct),
        ItemKind::Union(..) => Some(AdtKind::Union),
        ItemKind::Enum(..) => Some(AdtKind::Enum),
        _ => None,
    }
}

/// In a type definition, we check that to ensure that the types of the fields are well-formed.
fn check_type_defn<'tcx, F>(
    tcx: TyCtxt<'tcx>,
    item: &hir::Item<'tcx>,
    all_sized: bool,
    mut lookup_fields: F,
) where
    F: for<'fcx> FnMut(&FnCtxt<'fcx, 'tcx>) -> Vec<AdtVariant<'tcx>>,
{
    for_item(tcx, item).with_fcx(|fcx| {
        let variants = lookup_fields(fcx);
        let packed = tcx.adt_def(item.def_id).repr.packed();

        for variant in &variants {
            // For DST, or when drop needs to copy things around, all
            // intermediate types must be sized.
            let needs_drop_copy = || {
                packed && {
                    let ty = variant.fields.last().unwrap().ty;
                    let ty = tcx.erase_regions(ty);
                    if ty.needs_infer() {
                        tcx.sess
                            .delay_span_bug(item.span, &format!("inference variables in {:?}", ty));
                        // Just treat unresolved type expression as if it needs drop.
                        true
                    } else {
                        ty.needs_drop(tcx, tcx.param_env(item.def_id))
                    }
                }
            };
            let all_sized = all_sized || variant.fields.is_empty() || needs_drop_copy();
            let unsized_len = if all_sized { 0 } else { 1 };
            for (idx, field) in
                variant.fields[..variant.fields.len() - unsized_len].iter().enumerate()
            {
                let last = idx == variant.fields.len() - 1;
                fcx.register_bound(
                    field.ty,
                    tcx.require_lang_item(LangItem::Sized, None),
                    traits::ObligationCause::new(
                        field.span,
                        fcx.body_id,
                        traits::FieldSized {
                            adt_kind: match item_adt_kind(&item.kind) {
                                Some(i) => i,
                                None => bug!(),
                            },
                            span: field.span,
                            last,
                        },
                    ),
                );
            }

            // All field types must be well-formed.
            for field in &variant.fields {
                fcx.register_wf_obligation(
                    field.ty.into(),
                    field.span,
                    ObligationCauseCode::WellFormed(Some(WellFormedLoc::Ty(field.def_id))),
                )
            }

            // Explicit `enum` discriminant values must const-evaluate successfully.
            if let Some(discr_def_id) = variant.explicit_discr {
                let discr_substs = InternalSubsts::identity_for_item(tcx, discr_def_id.to_def_id());

                let cause = traits::ObligationCause::new(
                    tcx.def_span(discr_def_id),
                    fcx.body_id,
                    traits::MiscObligation,
                );
                fcx.register_predicate(traits::Obligation::new(
                    cause,
                    fcx.param_env,
                    ty::Binder::dummy(ty::PredicateKind::ConstEvaluatable(ty::Unevaluated::new(
                        ty::WithOptConstParam::unknown(discr_def_id.to_def_id()),
                        discr_substs,
                    )))
                    .to_predicate(tcx),
                ));
            }
        }

        check_where_clauses(fcx, item.span, item.def_id.to_def_id(), None);

        // No implied bounds in a struct definition.
        FxHashSet::default()
    });
}

#[instrument(skip(tcx, item))]
fn check_trait(tcx: TyCtxt<'_>, item: &hir::Item<'_>) {
    debug!(?item.def_id);

    let trait_def = tcx.trait_def(item.def_id);
    if trait_def.is_marker
        || matches!(trait_def.specialization_kind, TraitSpecializationKind::Marker)
    {
        for associated_def_id in &*tcx.associated_item_def_ids(item.def_id) {
            struct_span_err!(
                tcx.sess,
                tcx.def_span(*associated_def_id),
                E0714,
                "marker traits cannot have associated items",
            )
            .emit();
        }
    }

    // FIXME: this shouldn't use an `FnCtxt` at all.
    for_item(tcx, item).with_fcx(|fcx| {
        check_where_clauses(fcx, item.span, item.def_id.to_def_id(), None);

        FxHashSet::default()
    });
}

/// Checks all associated type defaults of trait `trait_def_id`.
///
/// Assuming the defaults are used, check that all predicates (bounds on the
/// assoc type and where clauses on the trait) hold.
fn check_associated_type_bounds(fcx: &FnCtxt<'_, '_>, item: &ty::AssocItem, span: Span) {
    let tcx = fcx.tcx;

    let bounds = tcx.explicit_item_bounds(item.def_id);

    debug!("check_associated_type_bounds: bounds={:?}", bounds);
    let wf_obligations = bounds.iter().flat_map(|&(bound, bound_span)| {
        let normalized_bound = fcx.normalize_associated_types_in(span, bound);
        traits::wf::predicate_obligations(
            fcx,
            fcx.param_env,
            fcx.body_id,
            normalized_bound,
            bound_span,
        )
    });

    for obligation in wf_obligations {
        debug!("next obligation cause: {:?}", obligation.cause);
        fcx.register_predicate(obligation);
    }
}

fn check_item_fn(
    tcx: TyCtxt<'_>,
    def_id: LocalDefId,
    ident: Ident,
    span: Span,
    decl: &hir::FnDecl<'_>,
) {
    for_id(tcx, def_id, span).with_fcx(|fcx| {
        let sig = tcx.fn_sig(def_id);
        let mut implied_bounds = FxHashSet::default();
        check_fn_or_method(fcx, ident.span, sig, decl, def_id.to_def_id(), &mut implied_bounds);
        implied_bounds
    })
}

fn check_item_type(tcx: TyCtxt<'_>, item_id: LocalDefId, ty_span: Span, allow_foreign_ty: bool) {
    debug!("check_item_type: {:?}", item_id);

    for_id(tcx, item_id, ty_span).with_fcx(|fcx| {
        let ty = tcx.type_of(item_id);
        let item_ty = fcx.normalize_associated_types_in_wf(ty_span, ty, WellFormedLoc::Ty(item_id));

        let mut forbid_unsized = true;
        if allow_foreign_ty {
            let tail = fcx.tcx.struct_tail_erasing_lifetimes(item_ty, fcx.param_env);
            if let ty::Foreign(_) = tail.kind() {
                forbid_unsized = false;
            }
        }

        fcx.register_wf_obligation(
            item_ty.into(),
            ty_span,
            ObligationCauseCode::WellFormed(Some(WellFormedLoc::Ty(item_id))),
        );
        if forbid_unsized {
            fcx.register_bound(
                item_ty,
                tcx.require_lang_item(LangItem::Sized, None),
                traits::ObligationCause::new(ty_span, fcx.body_id, traits::MiscObligation),
            );
        }

        // Ensure that the end result is `Sync` in a non-thread local `static`.
        let should_check_for_sync = tcx.static_mutability(item_id.to_def_id())
            == Some(hir::Mutability::Not)
            && !tcx.is_foreign_item(item_id.to_def_id())
            && !tcx.is_thread_local_static(item_id.to_def_id());

        if should_check_for_sync {
            fcx.register_bound(
                item_ty,
                tcx.require_lang_item(LangItem::Sync, Some(ty_span)),
                traits::ObligationCause::new(ty_span, fcx.body_id, traits::SharedStatic),
            );
        }

        // No implied bounds in a const, etc.
        FxHashSet::default()
    });
}

#[tracing::instrument(level = "debug", skip(tcx, ast_self_ty, ast_trait_ref))]
fn check_impl<'tcx>(
    tcx: TyCtxt<'tcx>,
    item: &'tcx hir::Item<'tcx>,
    ast_self_ty: &hir::Ty<'_>,
    ast_trait_ref: &Option<hir::TraitRef<'_>>,
) {
    for_item(tcx, item).with_fcx(|fcx| {
        match *ast_trait_ref {
            Some(ref ast_trait_ref) => {
                // `#[rustc_reservation_impl]` impls are not real impls and
                // therefore don't need to be WF (the trait's `Self: Trait` predicate
                // won't hold).
                let trait_ref = tcx.impl_trait_ref(item.def_id).unwrap();
                let trait_ref =
                    fcx.normalize_associated_types_in(ast_trait_ref.path.span, trait_ref);
                let obligations = traits::wf::trait_obligations(
                    fcx,
                    fcx.param_env,
                    fcx.body_id,
                    &trait_ref,
                    ast_trait_ref.path.span,
                    Some(item),
                );
                debug!(?obligations);
                for obligation in obligations {
                    fcx.register_predicate(obligation);
                }
            }
            None => {
                let self_ty = tcx.type_of(item.def_id);
                let self_ty = fcx.normalize_associated_types_in(item.span, self_ty);
                fcx.register_wf_obligation(
                    self_ty.into(),
                    ast_self_ty.span,
                    ObligationCauseCode::WellFormed(Some(WellFormedLoc::Ty(
                        item.hir_id().expect_owner(),
                    ))),
                );
            }
        }

        check_where_clauses(fcx, item.span, item.def_id.to_def_id(), None);

        fcx.impl_implied_bounds(item.def_id.to_def_id(), item.span)
    });
}

/// Checks where-clauses and inline bounds that are declared on `def_id`.
#[instrument(skip(fcx), level = "debug")]
fn check_where_clauses<'tcx, 'fcx>(
    fcx: &FnCtxt<'fcx, 'tcx>,
    span: Span,
    def_id: DefId,
    return_ty: Option<(Ty<'tcx>, Span)>,
) {
    let tcx = fcx.tcx;

    let predicates = tcx.predicates_of(def_id);
    let generics = tcx.generics_of(def_id);

    let is_our_default = |def: &ty::GenericParamDef| match def.kind {
        GenericParamDefKind::Type { has_default, .. }
        | GenericParamDefKind::Const { has_default } => {
            has_default && def.index >= generics.parent_count as u32
        }
        GenericParamDefKind::Lifetime => unreachable!(),
    };

    // Check that concrete defaults are well-formed. See test `type-check-defaults.rs`.
    // For example, this forbids the declaration:
    //
    //     struct Foo<T = Vec<[u32]>> { .. }
    //
    // Here, the default `Vec<[u32]>` is not WF because `[u32]: Sized` does not hold.
    for param in &generics.params {
        match param.kind {
            GenericParamDefKind::Type { .. } => {
                if is_our_default(param) {
                    let ty = tcx.type_of(param.def_id);
                    // Ignore dependent defaults -- that is, where the default of one type
                    // parameter includes another (e.g., `<T, U = T>`). In those cases, we can't
                    // be sure if it will error or not as user might always specify the other.
                    if !ty.definitely_needs_subst(tcx) {
                        fcx.register_wf_obligation(
                            ty.into(),
                            tcx.def_span(param.def_id),
                            ObligationCauseCode::MiscObligation,
                        );
                    }
                }
            }
            GenericParamDefKind::Const { .. } => {
                if is_our_default(param) {
                    // FIXME(const_generics_defaults): This
                    // is incorrect when dealing with unused substs, for example
                    // for `struct Foo<const N: usize, const M: usize = { 1 - 2 }>`
                    // we should eagerly error.
                    let default_ct = tcx.const_param_default(param.def_id);
                    if !default_ct.definitely_needs_subst(tcx) {
                        fcx.register_wf_obligation(
                            default_ct.into(),
                            tcx.def_span(param.def_id),
                            ObligationCauseCode::WellFormed(None),
                        );
                    }
                }
            }
            // Doesn't have defaults.
            GenericParamDefKind::Lifetime => {}
        }
    }

    // Check that trait predicates are WF when params are substituted by their defaults.
    // We don't want to overly constrain the predicates that may be written but we want to
    // catch cases where a default my never be applied such as `struct Foo<T: Copy = String>`.
    // Therefore we check if a predicate which contains a single type param
    // with a concrete default is WF with that default substituted.
    // For more examples see tests `defaults-well-formedness.rs` and `type-check-defaults.rs`.
    //
    // First we build the defaulted substitution.
    let substs = InternalSubsts::for_item(tcx, def_id, |param, _| {
        match param.kind {
            GenericParamDefKind::Lifetime => {
                // All regions are identity.
                tcx.mk_param_from_def(param)
            }

            GenericParamDefKind::Type { .. } => {
                // If the param has a default, ...
                if is_our_default(param) {
                    let default_ty = tcx.type_of(param.def_id);
                    // ... and it's not a dependent default, ...
                    if !default_ty.definitely_needs_subst(tcx) {
                        // ... then substitute it with the default.
                        return default_ty.into();
                    }
                }

                tcx.mk_param_from_def(param)
            }
            GenericParamDefKind::Const { .. } => {
                // If the param has a default, ...
                if is_our_default(param) {
                    let default_ct = tcx.const_param_default(param.def_id);
                    // ... and it's not a dependent default, ...
                    if !default_ct.definitely_needs_subst(tcx) {
                        // ... then substitute it with the default.
                        return default_ct.into();
                    }
                }

                tcx.mk_param_from_def(param)
            }
        }
    });

    // Now we build the substituted predicates.
    let default_obligations = predicates
        .predicates
        .iter()
        .flat_map(|&(pred, sp)| {
            struct CountParams<'tcx> {
                tcx: TyCtxt<'tcx>,
                params: FxHashSet<u32>,
            }
            impl<'tcx> ty::fold::TypeVisitor<'tcx> for CountParams<'tcx> {
                type BreakTy = ();
                fn tcx_for_anon_const_substs(&self) -> Option<TyCtxt<'tcx>> {
                    Some(self.tcx)
                }

                fn visit_ty(&mut self, t: Ty<'tcx>) -> ControlFlow<Self::BreakTy> {
                    if let ty::Param(param) = t.kind() {
                        self.params.insert(param.index);
                    }
                    t.super_visit_with(self)
                }

                fn visit_region(&mut self, _: ty::Region<'tcx>) -> ControlFlow<Self::BreakTy> {
                    ControlFlow::BREAK
                }

                fn visit_const(&mut self, c: &'tcx ty::Const<'tcx>) -> ControlFlow<Self::BreakTy> {
                    if let ty::ConstKind::Param(param) = c.val {
                        self.params.insert(param.index);
                    }
                    c.super_visit_with(self)
                }
            }
            let mut param_count = CountParams { tcx: fcx.tcx, params: FxHashSet::default() };
            let has_region = pred.visit_with(&mut param_count).is_break();
            let substituted_pred = pred.subst(tcx, substs);
            // Don't check non-defaulted params, dependent defaults (including lifetimes)
            // or preds with multiple params.
            if substituted_pred.definitely_has_param_types_or_consts(tcx)
                || param_count.params.len() > 1
                || has_region
            {
                None
            } else if predicates.predicates.iter().any(|&(p, _)| p == substituted_pred) {
                // Avoid duplication of predicates that contain no parameters, for example.
                None
            } else {
                Some((substituted_pred, sp))
            }
        })
        .map(|(pred, sp)| {
            // Convert each of those into an obligation. So if you have
            // something like `struct Foo<T: Copy = String>`, we would
            // take that predicate `T: Copy`, substitute to `String: Copy`
            // (actually that happens in the previous `flat_map` call),
            // and then try to prove it (in this case, we'll fail).
            //
            // Note the subtle difference from how we handle `predicates`
            // below: there, we are not trying to prove those predicates
            // to be *true* but merely *well-formed*.
            let pred = fcx.normalize_associated_types_in(sp, pred);
            let cause =
                traits::ObligationCause::new(sp, fcx.body_id, traits::ItemObligation(def_id));
            traits::Obligation::new(cause, fcx.param_env, pred)
        });

    let predicates = predicates.instantiate_identity(tcx);

    if let Some((return_ty, _)) = return_ty {
        if return_ty.has_infer_types_or_consts() {
            fcx.select_obligations_where_possible(false, |_| {});
        }
    }

    let predicates = fcx.normalize_associated_types_in(span, predicates);

    debug!(?predicates.predicates);
    assert_eq!(predicates.predicates.len(), predicates.spans.len());
    let wf_obligations =
        iter::zip(&predicates.predicates, &predicates.spans).flat_map(|(&p, &sp)| {
            traits::wf::predicate_obligations(fcx, fcx.param_env, fcx.body_id, p, sp)
        });

    for obligation in wf_obligations.chain(default_obligations) {
        debug!("next obligation cause: {:?}", obligation.cause);
        fcx.register_predicate(obligation);
    }
}

#[tracing::instrument(level = "debug", skip(fcx, span, hir_decl))]
fn check_fn_or_method<'fcx, 'tcx>(
    fcx: &FnCtxt<'fcx, 'tcx>,
    span: Span,
    sig: ty::PolyFnSig<'tcx>,
    hir_decl: &hir::FnDecl<'_>,
    def_id: DefId,
    implied_bounds: &mut FxHashSet<Ty<'tcx>>,
) {
    let sig = fcx.tcx.liberate_late_bound_regions(def_id, sig);

    // Normalize the input and output types one at a time, using a different
    // `WellFormedLoc` for each. We cannot call `normalize_associated_types`
    // on the entire `FnSig`, since this would use the same `WellFormedLoc`
    // for each type, preventing the HIR wf check from generating
    // a nice error message.
    let ty::FnSig { mut inputs_and_output, c_variadic, unsafety, abi } = sig;
    inputs_and_output =
        fcx.tcx.mk_type_list(inputs_and_output.iter().enumerate().map(|(i, ty)| {
            fcx.normalize_associated_types_in_wf(
                span,
                ty,
                WellFormedLoc::Param {
                    function: def_id.expect_local(),
                    // Note that the `param_idx` of the output type is
                    // one greater than the index of the last input type.
                    param_idx: i.try_into().unwrap(),
                },
            )
        }));
    // Manually call `normalize_assocaited_types_in` on the other types
    // in `FnSig`. This ensures that if the types of these fields
    // ever change to include projections, we will start normalizing
    // them automatically.
    let sig = ty::FnSig {
        inputs_and_output,
        c_variadic: fcx.normalize_associated_types_in(span, c_variadic),
        unsafety: fcx.normalize_associated_types_in(span, unsafety),
        abi: fcx.normalize_associated_types_in(span, abi),
    };

    for (i, (&input_ty, ty)) in iter::zip(sig.inputs(), hir_decl.inputs).enumerate() {
        fcx.register_wf_obligation(
            input_ty.into(),
            ty.span,
            ObligationCauseCode::WellFormed(Some(WellFormedLoc::Param {
                function: def_id.expect_local(),
                param_idx: i.try_into().unwrap(),
            })),
        );
    }

    implied_bounds.extend(sig.inputs());

    fcx.register_wf_obligation(
        sig.output().into(),
        hir_decl.output.span(),
        ObligationCauseCode::ReturnType,
    );

    // FIXME(#27579) return types should not be implied bounds
    implied_bounds.insert(sig.output());

    debug!(?implied_bounds);

    check_where_clauses(fcx, span, def_id, Some((sig.output(), hir_decl.output.span())));
}

const HELP_FOR_SELF_TYPE: &str = "consider changing to `self`, `&self`, `&mut self`, `self: Box<Self>`, \
     `self: Rc<Self>`, `self: Arc<Self>`, or `self: Pin<P>` (where P is one \
     of the previous types except `Self`)";

#[tracing::instrument(level = "debug", skip(fcx))]
fn check_method_receiver<'fcx, 'tcx>(
    fcx: &FnCtxt<'fcx, 'tcx>,
    fn_sig: &hir::FnSig<'_>,
    method: &ty::AssocItem,
    self_ty: Ty<'tcx>,
) {
    // Check that the method has a valid receiver type, given the type `Self`.
    debug!("check_method_receiver({:?}, self_ty={:?})", method, self_ty);

    if !method.fn_has_self_parameter {
        return;
    }

    let span = fn_sig.decl.inputs[0].span;

    let sig = fcx.tcx.fn_sig(method.def_id);
    let sig = fcx.tcx.liberate_late_bound_regions(method.def_id, sig);
    let sig = fcx.normalize_associated_types_in(span, sig);

    debug!("check_method_receiver: sig={:?}", sig);

    let self_ty = fcx.normalize_associated_types_in(span, self_ty);

    let receiver_ty = sig.inputs()[0];
    let receiver_ty = fcx.normalize_associated_types_in(span, receiver_ty);

    if fcx.tcx.features().arbitrary_self_types {
        if !receiver_is_valid(fcx, span, receiver_ty, self_ty, true) {
            // Report error; `arbitrary_self_types` was enabled.
            e0307(fcx, span, receiver_ty);
        }
    } else {
        if !receiver_is_valid(fcx, span, receiver_ty, self_ty, false) {
            if receiver_is_valid(fcx, span, receiver_ty, self_ty, true) {
                // Report error; would have worked with `arbitrary_self_types`.
                feature_err(
                    &fcx.tcx.sess.parse_sess,
                    sym::arbitrary_self_types,
                    span,
                    &format!(
                        "`{}` cannot be used as the type of `self` without \
                         the `arbitrary_self_types` feature",
                        receiver_ty,
                    ),
                )
                .help(HELP_FOR_SELF_TYPE)
                .emit();
            } else {
                // Report error; would not have worked with `arbitrary_self_types`.
                e0307(fcx, span, receiver_ty);
            }
        }
    }
}

fn e0307<'tcx>(fcx: &FnCtxt<'_, 'tcx>, span: Span, receiver_ty: Ty<'_>) {
    struct_span_err!(
        fcx.tcx.sess.diagnostic(),
        span,
        E0307,
        "invalid `self` parameter type: {}",
        receiver_ty,
    )
    .note("type of `self` must be `Self` or a type that dereferences to it")
    .help(HELP_FOR_SELF_TYPE)
    .emit();
}

/// Returns whether `receiver_ty` would be considered a valid receiver type for `self_ty`. If
/// `arbitrary_self_types` is enabled, `receiver_ty` must transitively deref to `self_ty`, possibly
/// through a `*const/mut T` raw pointer. If the feature is not enabled, the requirements are more
/// strict: `receiver_ty` must implement `Receiver` and directly implement
/// `Deref<Target = self_ty>`.
///
/// N.B., there are cases this function returns `true` but causes an error to be emitted,
/// particularly when `receiver_ty` derefs to a type that is the same as `self_ty` but has the
/// wrong lifetime. Be careful of this if you are calling this function speculatively.
fn receiver_is_valid<'fcx, 'tcx>(
    fcx: &FnCtxt<'fcx, 'tcx>,
    span: Span,
    receiver_ty: Ty<'tcx>,
    self_ty: Ty<'tcx>,
    arbitrary_self_types_enabled: bool,
) -> bool {
    let cause = fcx.cause(span, traits::ObligationCauseCode::MethodReceiver);

    let can_eq_self = |ty| fcx.infcx.can_eq(fcx.param_env, self_ty, ty).is_ok();

    // `self: Self` is always valid.
    if can_eq_self(receiver_ty) {
        if let Some(mut err) = fcx.demand_eqtype_with_origin(&cause, self_ty, receiver_ty) {
            err.emit();
        }
        return true;
    }

    let mut autoderef = fcx.autoderef(span, receiver_ty);

    // The `arbitrary_self_types` feature allows raw pointer receivers like `self: *const Self`.
    if arbitrary_self_types_enabled {
        autoderef = autoderef.include_raw_pointers();
    }

    // The first type is `receiver_ty`, which we know its not equal to `self_ty`; skip it.
    autoderef.next();

    let receiver_trait_def_id = fcx.tcx.require_lang_item(LangItem::Receiver, None);

    // Keep dereferencing `receiver_ty` until we get to `self_ty`.
    loop {
        if let Some((potential_self_ty, _)) = autoderef.next() {
            debug!(
                "receiver_is_valid: potential self type `{:?}` to match `{:?}`",
                potential_self_ty, self_ty
            );

            if can_eq_self(potential_self_ty) {
                fcx.register_predicates(autoderef.into_obligations());

                if let Some(mut err) =
                    fcx.demand_eqtype_with_origin(&cause, self_ty, potential_self_ty)
                {
                    err.emit();
                }

                break;
            } else {
                // Without `feature(arbitrary_self_types)`, we require that each step in the
                // deref chain implement `receiver`
                if !arbitrary_self_types_enabled
                    && !receiver_is_implemented(
                        fcx,
                        receiver_trait_def_id,
                        cause.clone(),
                        potential_self_ty,
                    )
                {
                    return false;
                }
            }
        } else {
            debug!("receiver_is_valid: type `{:?}` does not deref to `{:?}`", receiver_ty, self_ty);
            // If he receiver already has errors reported due to it, consider it valid to avoid
            // unnecessary errors (#58712).
            return receiver_ty.references_error();
        }
    }

    // Without `feature(arbitrary_self_types)`, we require that `receiver_ty` implements `Receiver`.
    if !arbitrary_self_types_enabled
        && !receiver_is_implemented(fcx, receiver_trait_def_id, cause.clone(), receiver_ty)
    {
        return false;
    }

    true
}

fn receiver_is_implemented<'tcx>(
    fcx: &FnCtxt<'_, 'tcx>,
    receiver_trait_def_id: DefId,
    cause: ObligationCause<'tcx>,
    receiver_ty: Ty<'tcx>,
) -> bool {
    let trait_ref = ty::Binder::dummy(ty::TraitRef {
        def_id: receiver_trait_def_id,
        substs: fcx.tcx.mk_substs_trait(receiver_ty, &[]),
    });

    let obligation = traits::Obligation::new(
        cause,
        fcx.param_env,
        trait_ref.without_const().to_predicate(fcx.tcx),
    );

    if fcx.predicate_must_hold_modulo_regions(&obligation) {
        true
    } else {
        debug!(
            "receiver_is_implemented: type `{:?}` does not implement `Receiver` trait",
            receiver_ty
        );
        false
    }
}

fn check_variances_for_type_defn<'tcx>(
    tcx: TyCtxt<'tcx>,
    item: &hir::Item<'tcx>,
    hir_generics: &hir::Generics<'_>,
) {
    let ty = tcx.type_of(item.def_id);
    if tcx.has_error_field(ty) {
        return;
    }

    let ty_predicates = tcx.predicates_of(item.def_id);
    assert_eq!(ty_predicates.parent, None);
    let variances = tcx.variances_of(item.def_id);

    let mut constrained_parameters: FxHashSet<_> = variances
        .iter()
        .enumerate()
        .filter(|&(_, &variance)| variance != ty::Bivariant)
        .map(|(index, _)| Parameter(index as u32))
        .collect();

    identify_constrained_generic_params(tcx, ty_predicates, None, &mut constrained_parameters);

    for (index, _) in variances.iter().enumerate() {
        if constrained_parameters.contains(&Parameter(index as u32)) {
            continue;
        }

        let param = &hir_generics.params[index];

        match param.name {
            hir::ParamName::Error => {}
            _ => report_bivariance(tcx, param),
        }
    }
}

fn report_bivariance(tcx: TyCtxt<'_>, param: &rustc_hir::GenericParam<'_>) {
    let span = param.span;
    let param_name = param.name.ident().name;
    let mut err = error_392(tcx, span, param_name);

    let suggested_marker_id = tcx.lang_items().phantom_data();
    // Help is available only in presence of lang items.
    let msg = if let Some(def_id) = suggested_marker_id {
        format!(
            "consider removing `{}`, referring to it in a field, or using a marker such as `{}`",
            param_name,
            tcx.def_path_str(def_id),
        )
    } else {
        format!("consider removing `{}` or referring to it in a field", param_name)
    };
    err.help(&msg);

    if matches!(param.kind, rustc_hir::GenericParamKind::Type { .. }) {
        err.help(&format!(
            "if you intended `{0}` to be a const parameter, use `const {0}: usize` instead",
            param_name
        ));
    }
    err.emit()
}

/// Feature gates RFC 2056 -- trivial bounds, checking for global bounds that
/// aren't true.
fn check_false_global_bounds(fcx: &FnCtxt<'_, '_>, mut span: Span, id: hir::HirId) {
    let empty_env = ty::ParamEnv::empty();

    let def_id = fcx.tcx.hir().local_def_id(id);
    let predicates_with_span =
        fcx.tcx.predicates_of(def_id).predicates.iter().map(|(p, span)| (*p, *span));
    // Check elaborated bounds.
    let implied_obligations = traits::elaborate_predicates_with_span(fcx.tcx, predicates_with_span);

    for obligation in implied_obligations {
        let pred = obligation.predicate;
        // Match the existing behavior.
        if pred.is_global(fcx.tcx) && !pred.has_late_bound_regions() {
            let pred = fcx.normalize_associated_types_in(span, pred);
            let hir_node = fcx.tcx.hir().find(id);

            // only use the span of the predicate clause (#90869)

            if let Some(hir::Generics { where_clause, .. }) =
                hir_node.and_then(|node| node.generics())
            {
                let obligation_span = obligation.cause.span(fcx.tcx);

                span = where_clause
                    .predicates
                    .iter()
                    // There seems to be no better way to find out which predicate we are in
                    .find(|pred| pred.span().contains(obligation_span))
                    .map(|pred| pred.span())
                    .unwrap_or(obligation_span);
            }

            let obligation = traits::Obligation::new(
                traits::ObligationCause::new(span, id, traits::TrivialBound),
                empty_env,
                pred,
            );
            fcx.register_predicate(obligation);
        }
    }

    fcx.select_all_obligations_or_error();
}

#[derive(Clone, Copy)]
pub struct CheckTypeWellFormedVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> CheckTypeWellFormedVisitor<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> CheckTypeWellFormedVisitor<'tcx> {
        CheckTypeWellFormedVisitor { tcx }
    }
}

impl<'tcx> ParItemLikeVisitor<'tcx> for CheckTypeWellFormedVisitor<'tcx> {
    fn visit_item(&self, i: &'tcx hir::Item<'tcx>) {
        Visitor::visit_item(&mut self.clone(), i);
    }

    fn visit_trait_item(&self, trait_item: &'tcx hir::TraitItem<'tcx>) {
        Visitor::visit_trait_item(&mut self.clone(), trait_item);
    }

    fn visit_impl_item(&self, impl_item: &'tcx hir::ImplItem<'tcx>) {
        Visitor::visit_impl_item(&mut self.clone(), impl_item);
    }

    fn visit_foreign_item(&self, foreign_item: &'tcx hir::ForeignItem<'tcx>) {
        Visitor::visit_foreign_item(&mut self.clone(), foreign_item)
    }
}

impl<'tcx> Visitor<'tcx> for CheckTypeWellFormedVisitor<'tcx> {
    type Map = hir_map::Map<'tcx>;

    fn nested_visit_map(&mut self) -> hir_visit::NestedVisitorMap<Self::Map> {
        hir_visit::NestedVisitorMap::OnlyBodies(self.tcx.hir())
    }

    #[instrument(skip(self, i), level = "debug")]
    fn visit_item(&mut self, i: &'tcx hir::Item<'tcx>) {
        trace!(?i);
        self.tcx.ensure().check_item_well_formed(i.def_id);
        hir_visit::walk_item(self, i);
    }

    #[instrument(skip(self, trait_item), level = "debug")]
    fn visit_trait_item(&mut self, trait_item: &'tcx hir::TraitItem<'tcx>) {
        trace!(?trait_item);
        self.tcx.ensure().check_trait_item_well_formed(trait_item.def_id);
        hir_visit::walk_trait_item(self, trait_item);
    }

    #[instrument(skip(self, impl_item), level = "debug")]
    fn visit_impl_item(&mut self, impl_item: &'tcx hir::ImplItem<'tcx>) {
        trace!(?impl_item);
        self.tcx.ensure().check_impl_item_well_formed(impl_item.def_id);
        hir_visit::walk_impl_item(self, impl_item);
    }

    fn visit_generic_param(&mut self, p: &'tcx hir::GenericParam<'tcx>) {
        check_param_wf(self.tcx, p);
        hir_visit::walk_generic_param(self, p);
    }
}

///////////////////////////////////////////////////////////////////////////
// ADT

// FIXME(eddyb) replace this with getting fields/discriminants through `ty::AdtDef`.
struct AdtVariant<'tcx> {
    /// Types of fields in the variant, that must be well-formed.
    fields: Vec<AdtField<'tcx>>,

    /// Explicit discriminant of this variant (e.g. `A = 123`),
    /// that must evaluate to a constant value.
    explicit_discr: Option<LocalDefId>,
}

struct AdtField<'tcx> {
    ty: Ty<'tcx>,
    def_id: LocalDefId,
    span: Span,
}

impl<'a, 'tcx> FnCtxt<'a, 'tcx> {
    // FIXME(eddyb) replace this with getting fields through `ty::AdtDef`.
    fn non_enum_variant(&self, struct_def: &hir::VariantData<'_>) -> AdtVariant<'tcx> {
        let fields = struct_def
            .fields()
            .iter()
            .map(|field| {
                let def_id = self.tcx.hir().local_def_id(field.hir_id);
                let field_ty = self.tcx.type_of(def_id);
                let field_ty = self.normalize_associated_types_in(field.ty.span, field_ty);
                let field_ty = self.resolve_vars_if_possible(field_ty);
                debug!("non_enum_variant: type of field {:?} is {:?}", field, field_ty);
                AdtField { ty: field_ty, span: field.ty.span, def_id }
            })
            .collect();
        AdtVariant { fields, explicit_discr: None }
    }

    fn enum_variants(&self, enum_def: &hir::EnumDef<'_>) -> Vec<AdtVariant<'tcx>> {
        enum_def
            .variants
            .iter()
            .map(|variant| AdtVariant {
                fields: self.non_enum_variant(&variant.data).fields,
                explicit_discr: variant
                    .disr_expr
                    .map(|explicit_discr| self.tcx.hir().local_def_id(explicit_discr.hir_id)),
            })
            .collect()
    }

    pub(super) fn impl_implied_bounds(
        &self,
        impl_def_id: DefId,
        span: Span,
    ) -> FxHashSet<Ty<'tcx>> {
        match self.tcx.impl_trait_ref(impl_def_id) {
            Some(trait_ref) => {
                // Trait impl: take implied bounds from all types that
                // appear in the trait reference.
                let trait_ref = self.normalize_associated_types_in(span, trait_ref);
                trait_ref.substs.types().collect()
            }

            None => {
                // Inherent impl: take implied bounds from the `self` type.
                let self_ty = self.tcx.type_of(impl_def_id);
                let self_ty = self.normalize_associated_types_in(span, self_ty);
                FxHashSet::from_iter([self_ty])
            }
        }
    }
}

fn error_392(tcx: TyCtxt<'_>, span: Span, param_name: Symbol) -> DiagnosticBuilder<'_> {
    let mut err =
        struct_span_err!(tcx.sess, span, E0392, "parameter `{}` is never used", param_name);
    err.span_label(span, "unused parameter");
    err
}
