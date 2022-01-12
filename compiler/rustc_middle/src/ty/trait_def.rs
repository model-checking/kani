use crate::traits::specialization_graph;
use crate::ty::fast_reject::{self, SimplifiedType, SimplifyParams, StripReferences};
use crate::ty::fold::TypeFoldable;
use crate::ty::{Ty, TyCtxt};
use rustc_hir as hir;
use rustc_hir::def_id::DefId;
use rustc_hir::definitions::DefPathHash;

use rustc_data_structures::fx::FxIndexMap;
use rustc_errors::ErrorReported;
use rustc_macros::HashStable;

/// A trait's definition with type information.
#[derive(HashStable)]
pub struct TraitDef {
    // We already have the def_path_hash below, no need to hash it twice
    #[stable_hasher(ignore)]
    pub def_id: DefId,

    pub unsafety: hir::Unsafety,

    /// If `true`, then this trait had the `#[rustc_paren_sugar]`
    /// attribute, indicating that it should be used with `Foo()`
    /// sugar. This is a temporary thing -- eventually any trait will
    /// be usable with the sugar (or without it).
    pub paren_sugar: bool,

    pub has_auto_impl: bool,

    /// If `true`, then this trait has the `#[marker]` attribute, indicating
    /// that all its associated items have defaults that cannot be overridden,
    /// and thus `impl`s of it are allowed to overlap.
    pub is_marker: bool,

    /// If `true`, then this trait has the `#[rustc_skip_array_during_method_dispatch]`
    /// attribute, indicating that editions before 2021 should not consider this trait
    /// during method dispatch if the receiver is an array.
    pub skip_array_during_method_dispatch: bool,

    /// Used to determine whether the standard library is allowed to specialize
    /// on this trait.
    pub specialization_kind: TraitSpecializationKind,

    /// The ICH of this trait's DefPath, cached here so it doesn't have to be
    /// recomputed all the time.
    pub def_path_hash: DefPathHash,
}

/// Whether this trait is treated specially by the standard library
/// specialization lint.
#[derive(HashStable, PartialEq, Clone, Copy, TyEncodable, TyDecodable)]
pub enum TraitSpecializationKind {
    /// The default. Specializing on this trait is not allowed.
    None,
    /// Specializing on this trait is allowed because it doesn't have any
    /// methods. For example `Sized` or `FusedIterator`.
    /// Applies to traits with the `rustc_unsafe_specialization_marker`
    /// attribute.
    Marker,
    /// Specializing on this trait is allowed because all of the impls of this
    /// trait are "always applicable". Always applicable means that if
    /// `X<'x>: T<'y>` for any lifetimes, then `for<'a, 'b> X<'a>: T<'b>`.
    /// Applies to traits with the `rustc_specialization_trait` attribute.
    AlwaysApplicable,
}

#[derive(Default, Debug, HashStable)]
pub struct TraitImpls {
    blanket_impls: Vec<DefId>,
    /// Impls indexed by their simplified self type, for fast lookup.
    non_blanket_impls: FxIndexMap<SimplifiedType, Vec<DefId>>,
}

impl TraitImpls {
    pub fn blanket_impls(&self) -> &[DefId] {
        self.blanket_impls.as_slice()
    }
}

impl<'tcx> TraitDef {
    pub fn new(
        def_id: DefId,
        unsafety: hir::Unsafety,
        paren_sugar: bool,
        has_auto_impl: bool,
        is_marker: bool,
        skip_array_during_method_dispatch: bool,
        specialization_kind: TraitSpecializationKind,
        def_path_hash: DefPathHash,
    ) -> TraitDef {
        TraitDef {
            def_id,
            unsafety,
            paren_sugar,
            has_auto_impl,
            is_marker,
            skip_array_during_method_dispatch,
            specialization_kind,
            def_path_hash,
        }
    }

    pub fn ancestors(
        &self,
        tcx: TyCtxt<'tcx>,
        of_impl: DefId,
    ) -> Result<specialization_graph::Ancestors<'tcx>, ErrorReported> {
        specialization_graph::ancestors(tcx, self.def_id, of_impl)
    }
}

impl<'tcx> TyCtxt<'tcx> {
    pub fn for_each_impl<F: FnMut(DefId)>(self, def_id: DefId, mut f: F) {
        let impls = self.trait_impls_of(def_id);

        for &impl_def_id in impls.blanket_impls.iter() {
            f(impl_def_id);
        }

        for v in impls.non_blanket_impls.values() {
            for &impl_def_id in v {
                f(impl_def_id);
            }
        }
    }

    /// Iterate over every impl that could possibly match the
    /// self type `self_ty`.
    pub fn for_each_relevant_impl<F: FnMut(DefId)>(
        self,
        def_id: DefId,
        self_ty: Ty<'tcx>,
        mut f: F,
    ) {
        let _: Option<()> = self.find_map_relevant_impl(def_id, self_ty, |did| {
            f(did);
            None
        });
    }

    /// Applies function to every impl that could possibly match the self type `self_ty` and returns
    /// the first non-none value.
    pub fn find_map_relevant_impl<T, F: FnMut(DefId) -> Option<T>>(
        self,
        def_id: DefId,
        self_ty: Ty<'tcx>,
        mut f: F,
    ) -> Option<T> {
        // FIXME: This depends on the set of all impls for the trait. That is
        // unfortunate wrt. incremental compilation.
        //
        // If we want to be faster, we could have separate queries for
        // blanket and non-blanket impls, and compare them separately.
        let impls = self.trait_impls_of(def_id);

        for &impl_def_id in impls.blanket_impls.iter() {
            if let result @ Some(_) = f(impl_def_id) {
                return result;
            }
        }

        // Note that we're using `SimplifyParams::Yes` to query `non_blanket_impls` while using
        // `SimplifyParams::No` while actually adding them.
        //
        // This way, when searching for some impl for `T: Trait`, we do not look at any impls
        // whose outer level is not a parameter or projection. Especially for things like
        // `T: Clone` this is incredibly useful as we would otherwise look at all the impls
        // of `Clone` for `Option<T>`, `Vec<T>`, `ConcreteType` and so on.
        if let Some(simp) =
            fast_reject::simplify_type(self, self_ty, SimplifyParams::Yes, StripReferences::No)
        {
            if let Some(impls) = impls.non_blanket_impls.get(&simp) {
                for &impl_def_id in impls {
                    if let result @ Some(_) = f(impl_def_id) {
                        return result;
                    }
                }
            }
        } else {
            for &impl_def_id in impls.non_blanket_impls.values().flatten() {
                if let result @ Some(_) = f(impl_def_id) {
                    return result;
                }
            }
        }

        None
    }

    /// Returns an iterator containing all impls
    pub fn all_impls(self, def_id: DefId) -> impl Iterator<Item = DefId> + 'tcx {
        let TraitImpls { blanket_impls, non_blanket_impls } = self.trait_impls_of(def_id);

        blanket_impls.iter().chain(non_blanket_impls.iter().map(|(_, v)| v).flatten()).cloned()
    }
}

// Query provider for `trait_impls_of`.
pub(super) fn trait_impls_of_provider(tcx: TyCtxt<'_>, trait_id: DefId) -> TraitImpls {
    let mut impls = TraitImpls::default();

    // Traits defined in the current crate can't have impls in upstream
    // crates, so we don't bother querying the cstore.
    if !trait_id.is_local() {
        for &cnum in tcx.crates(()).iter() {
            for &(impl_def_id, simplified_self_ty) in
                tcx.implementations_of_trait((cnum, trait_id)).iter()
            {
                if let Some(simplified_self_ty) = simplified_self_ty {
                    impls
                        .non_blanket_impls
                        .entry(simplified_self_ty)
                        .or_default()
                        .push(impl_def_id);
                } else {
                    impls.blanket_impls.push(impl_def_id);
                }
            }
        }
    }

    for &impl_def_id in tcx.hir().trait_impls(trait_id) {
        let impl_def_id = impl_def_id.to_def_id();

        let impl_self_ty = tcx.type_of(impl_def_id);
        if impl_self_ty.references_error() {
            continue;
        }

        if let Some(simplified_self_ty) =
            fast_reject::simplify_type(tcx, impl_self_ty, SimplifyParams::No, StripReferences::No)
        {
            impls.non_blanket_impls.entry(simplified_self_ty).or_default().push(impl_def_id);
        } else {
            impls.blanket_impls.push(impl_def_id);
        }
    }

    impls
}
