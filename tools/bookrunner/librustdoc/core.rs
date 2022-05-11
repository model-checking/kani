// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def::Res;
use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{HirId, Path};
use rustc_middle::hir::nested_filter;
use rustc_middle::ty::{ParamEnv, TyCtxt};
use rustc_session::Session;

use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use crate::clean::{self, ItemId};
use crate::config::RenderOptions;
use crate::formats::cache::Cache;

crate struct DocContext<'tcx> {
    crate tcx: TyCtxt<'tcx>,
    /// Used for normalization.
    ///
    /// Most of this logic is copied from rustc_lint::late.
    crate param_env: ParamEnv<'tcx>,
    /// Later on moved through `clean::Crate` into `cache`
    crate external_traits: Rc<RefCell<FxHashMap<DefId, clean::TraitWithExtraInfo>>>,
    /// Used while populating `external_traits` to ensure we don't process the same trait twice at
    /// the same time.
    crate active_extern_traits: FxHashSet<DefId>,
    // The current set of parameter substitutions,
    // for expanding type aliases at the HIR level:
    /// Table `DefId` of type, lifetime, or const parameter -> substituted type, lifetime, or const
    crate substs: FxHashMap<DefId, clean::SubstParam>,
    /// Table synthetic type parameter for `impl Trait` in argument position -> bounds
    crate impl_trait_bounds: FxHashMap<ImplTraitParam, Vec<clean::GenericBound>>,
    /// The options given to rustdoc that could be relevant to a pass.
    crate render_options: RenderOptions,
    /// This same cache is used throughout rustdoc, including in [`crate::html::render`].
    crate cache: Cache,
    /// Used by [`clean::inline`] to tell if an item has already been inlined.
    crate inlined: FxHashSet<ItemId>,
}

impl<'tcx> DocContext<'tcx> {
    crate fn sess(&self) -> &'tcx Session {
        self.tcx.sess
    }

    crate fn with_param_env<T, F: FnOnce(&mut Self) -> T>(&mut self, def_id: DefId, f: F) -> T {
        let old_param_env = mem::replace(&mut self.param_env, self.tcx.param_env(def_id));
        let ret = f(self);
        self.param_env = old_param_env;
        ret
    }

    /// Call the closure with the given parameters set as
    /// the substitutions for a type alias' RHS.
    crate fn enter_alias<F, R>(&mut self, substs: FxHashMap<DefId, clean::SubstParam>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let old_substs = mem::replace(&mut self.substs, substs);
        let r = f(self);
        self.substs = old_substs;
        r
    }

    /// Like `hir().local_def_id_to_hir_id()`, but skips calling it on fake DefIds.
    /// (This avoids a slice-index-out-of-bounds panic.)
    crate fn as_local_hir_id(tcx: TyCtxt<'_>, def_id: ItemId) -> Option<HirId> {
        match def_id {
            ItemId::DefId(real_id) => {
                real_id.as_local().map(|def_id| tcx.hir().local_def_id_to_hir_id(def_id))
            }
            // FIXME: Can this be `Some` for `Auto` or `Blanket`?
            _ => None,
        }
    }
}

/// Due to <https://github.com/rust-lang/rust/pull/73566>,
/// the name resolution pass may find errors that are never emitted.
/// If typeck is called after this happens, then we'll get an ICE:
/// 'Res::Error found but not reported'. To avoid this, emit the errors now.
struct EmitIgnoredResolutionErrors<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> EmitIgnoredResolutionErrors<'tcx> {}

impl<'tcx> Visitor<'tcx> for EmitIgnoredResolutionErrors<'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        // We need to recurse into nested closures,
        // since those will fallback to the parent for type checking.
        self.tcx.hir()
    }

    fn visit_path(&mut self, path: &'tcx Path<'_>, _id: HirId) {
        debug!("visiting path {:?}", path);
        if path.res == Res::Err {
            // We have less context here than in rustc_resolve,
            // so we can only emit the name and span.
            // However we can give a hint that rustc_resolve will have more info.
            let label = format!(
                "could not resolve path `{}`",
                path.segments
                    .iter()
                    .map(|segment| segment.ident.as_str())
                    .intersperse("::")
                    .collect::<String>()
            );
            let mut err = rustc_errors::struct_span_err!(
                self.tcx.sess,
                path.span,
                E0433,
                "failed to resolve: {}",
                label
            );
            err.span_label(path.span, label);
            err.note("this error was originally ignored because you are running `rustdoc`");
            err.note("try running again with `rustc` or `cargo check` and you may get a more detailed error");
            err.emit();
        }
        // We could have an outer resolution that succeeded,
        // but with generic parameters that failed.
        // Recurse into the segments so we catch those too.
        intravisit::walk_path(self, path);
    }
}

/// `DefId` or parameter index (`ty::ParamTy.index`) of a synthetic type parameter
/// for `impl Trait` in argument position.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
crate enum ImplTraitParam {
    DefId(DefId),
    ParamIndex(u32),
}

impl From<DefId> for ImplTraitParam {
    fn from(did: DefId) -> Self {
        ImplTraitParam::DefId(did)
    }
}

impl From<u32> for ImplTraitParam {
    fn from(idx: u32) -> Self {
        ImplTraitParam::ParamIndex(idx)
    }
}
