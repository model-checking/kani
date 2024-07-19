// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for resolving strings representing simple paths to
//! `DefId`s for functions and methods. For the definition of a simple path, see
//! <https://doc.rust-lang.org/reference/paths.html#simple-paths>.
//!
//! TODO: Extend this logic to support resolving qualified paths.
//! <https://github.com/model-checking/kani/issues/1997>
//!
//! Note that glob use statements can form loops. The paths can also walk through the loop.

use rustc_smir::rustc_internal;
use std::collections::HashSet;
use std::fmt;
use std::iter::Peekable;

use rustc_errors::ErrorGuaranteed;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId, LocalModDefId, CRATE_DEF_INDEX, LOCAL_CRATE};
use rustc_hir::{ItemKind, UseKind};
use rustc_middle::ty::TyCtxt;
use stable_mir::ty::{FnDef, RigidTy, TyKind};
use stable_mir::CrateDef;
use tracing::debug;

/// Attempts to resolve a simple path (in the form of a string) to a function / method `DefId`.
///
/// TODO: Extend this implementation to handle qualified paths and simple paths
/// corresponding to trait methods.
/// <https://github.com/model-checking/kani/issues/1997>
pub fn resolve_fn<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    path_str: &str,
) -> Result<DefId, ResolveError<'tcx>> {
    let result = resolve_path(tcx, current_module, path_str);
    match result {
        Ok(def_id) => {
            let def_kind = tcx.def_kind(def_id);
            if matches!(def_kind, DefKind::AssocFn | DefKind::Fn) {
                Ok(def_id)
            } else {
                Err(ResolveError::UnexpectedType {
                    tcx,
                    item: def_id,
                    expected: "function / method",
                })
            }
        }
        err => err,
    }
}

/// Resolve the name of a function from the context of the definition provided.
///
/// Ideally this should pass a more precise span, but we don't keep them around.
pub fn expect_resolve_fn<T: CrateDef>(
    tcx: TyCtxt,
    res_cx: T,
    name: &str,
    reason: &str,
) -> Result<FnDef, ErrorGuaranteed> {
    let internal_def_id = rustc_internal::internal(tcx, res_cx.def_id());
    let current_module = tcx.parent_module_from_def_id(internal_def_id.as_local().unwrap());
    let maybe_resolved = resolve_fn(tcx, current_module.to_local_def_id(), name);
    let resolved = maybe_resolved.map_err(|err| {
        tcx.dcx().span_err(
            rustc_internal::internal(tcx, res_cx.span()),
            format!("Failed to resolve `{name}` for `{reason}`: {err}"),
        )
    })?;
    let ty_internal = tcx.type_of(resolved).instantiate_identity();
    let ty = rustc_internal::stable(ty_internal);
    if let TyKind::RigidTy(RigidTy::FnDef(def, _)) = ty.kind() {
        Ok(def)
    } else {
        unreachable!("Expected function for `{name}`, but found: {ty}")
    }
}

/// Attempts to resolve a simple path (in the form of a string) to a `DefId`.
/// The current module is provided as an argument in order to resolve relative
/// paths.
///
/// Note: This function was written to be generic, however, it has only been tested for functions.
pub(crate) fn resolve_path<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    path_str: &str,
) -> Result<DefId, ResolveError<'tcx>> {
    let _span = tracing::span!(tracing::Level::DEBUG, "path_resolution").entered();

    let path = resolve_prefix(tcx, current_module, path_str)?;
    path.segments.into_iter().try_fold(path.base, |base, name| {
        debug!(?base, ?name, "resolve_path");
        let def_kind = tcx.def_kind(base);
        let next_item = match def_kind {
            DefKind::ForeignMod | DefKind::Mod => resolve_in_module(tcx, base, &name),
            DefKind::Struct | DefKind::Enum | DefKind::Union => resolve_in_type(tcx, base, &name),
            kind => {
                debug!(?base, ?kind, "resolve_path: unexpected item");
                Err(ResolveError::UnexpectedType { tcx, item: base, expected: "module" })
            }
        };
        next_item
    })
}

/// Provide information about where the resolution failed.
/// Todo: Add error message.
pub enum ResolveError<'tcx> {
    /// Ambiguous glob resolution.
    AmbiguousGlob { tcx: TyCtxt<'tcx>, name: String, base: DefId, candidates: Vec<DefId> },
    /// Use super past the root of a crate.
    ExtraSuper,
    /// Invalid path.
    InvalidPath { msg: String },
    /// Unable to find an item.
    MissingItem { tcx: TyCtxt<'tcx>, base: DefId, unresolved: String },
    /// Error triggered when the identifier points to an item with unexpected type.
    UnexpectedType { tcx: TyCtxt<'tcx>, item: DefId, expected: &'static str },
}

impl<'tcx> fmt::Debug for ResolveError<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl<'tcx> fmt::Display for ResolveError<'tcx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::ExtraSuper => {
                write!(f, "there are too many leading `super` keywords")
            }
            ResolveError::AmbiguousGlob { tcx, base, name, candidates } => {
                let location = description(*tcx, *base);
                write!(
                    f,
                    "`{name}` is ambiguous because of multiple glob imports in {location}. Found:\n{}",
                    candidates
                        .iter()
                        .map(|def_id| tcx.def_path_str(*def_id))
                        .intersperse("\n".to_string())
                        .collect::<String>()
                )
            }
            ResolveError::InvalidPath { msg } => write!(f, "{msg}"),
            ResolveError::UnexpectedType { tcx, item: def_id, expected } => write!(
                f,
                "expected {expected}, found {} `{}`",
                tcx.def_kind(def_id).descr(*def_id),
                tcx.def_path_str(*def_id)
            ),
            ResolveError::MissingItem { tcx, base, unresolved } => {
                let def_desc = description(*tcx, *base);
                write!(f, "unable to find `{unresolved}` inside {def_desc}")
            }
        }
    }
}

/// The segments of a path.
type Segments = Vec<String>;

/// A path consisting of a starting point and a bunch of segments. If `base`
/// matches `Base::LocalModule { id: _, may_be_external_path : true }`, then
/// `segments` cannot be empty.
#[derive(Debug, Hash)]
struct Path {
    pub base: DefId,
    pub segments: Segments,
}

/// Identifier for the top module of the crate.
const CRATE: &str = "crate";
/// rustc represents initial `::` as `{{root}}`.
const ROOT: &str = "{{root}}";
/// Identifier for the current module.
const SELF: &str = "self";
/// Identifier for the parent of the current module.
const SUPER: &str = "super";

/// Takes a string representation of a path and turns it into a `Path` data
/// structure, resolving prefix qualifiers (like `crate`, `self`, etc.) along the way.
fn resolve_prefix<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    name: &str,
) -> Result<Path, ResolveError<'tcx>> {
    debug!(?name, ?current_module, "resolve_prefix");

    // Split the string into segments separated by `::`. Trim the whitespace
    // since path strings generated from macros sometimes add spaces around
    // `::`.
    let mut segments = name.split("::").map(|s| s.trim().to_string()).peekable();
    assert!(segments.peek().is_some(), "expected identifier, found `{name}`");

    // Resolve qualifiers `crate`, initial `::`, and `self`. The qualifier
    // `self` may be followed be `super` (handled below).
    let first = segments.peek().unwrap().as_str();
    match first {
        ROOT => {
            // Skip root and get the external crate from the name that follows `::`.
            let next = segments.nth(1);
            if let Some(next_name) = next {
                let result = resolve_external(tcx, &next_name);
                if let Some(def_id) = result {
                    Ok(Path { base: def_id, segments: segments.collect() })
                } else {
                    Err(ResolveError::MissingItem {
                        tcx,
                        base: current_module.to_def_id(),
                        unresolved: next_name,
                    })
                }
            } else {
                Err(ResolveError::InvalidPath { msg: "expected identifier after `::`".to_string() })
            }
        }
        CRATE => {
            segments.next();
            // Find the module at the root of the crate.
            let current_module_hir_id = tcx.local_def_id_to_hir_id(current_module);
            let crate_root = match tcx.hir().parent_iter(current_module_hir_id).last() {
                None => current_module,
                Some((hir_id, _)) => hir_id.owner.def_id,
            };
            Ok(Path { base: crate_root.to_def_id(), segments: segments.collect() })
        }
        SELF => {
            segments.next();
            resolve_super(tcx, current_module, segments)
        }
        SUPER => resolve_super(tcx, current_module, segments),
        _ => {
            // No special key word was used. Try local first otherwise try external name.
            let next_name = segments.next().unwrap();
            let def_id =
                resolve_in_module(tcx, current_module.to_def_id(), &next_name).or_else(|err| {
                    if matches!(err, ResolveError::MissingItem { .. }) {
                        // Only try external if we couldn't find anything.
                        resolve_external(tcx, &next_name).ok_or(err)
                    } else {
                        Err(err)
                    }
                })?;
            Ok(Path { base: def_id, segments: segments.collect() })
        }
    }
}

/// Pop up the module stack until we account for all the `super` prefixes.
/// This method will error out if it tries to backtrace from the root crate.
fn resolve_super<'tcx, I>(
    tcx: TyCtxt,
    current_module: LocalDefId,
    mut segments: Peekable<I>,
) -> Result<Path, ResolveError<'tcx>>
where
    I: Iterator<Item = String>,
{
    let current_module_hir_id = tcx.local_def_id_to_hir_id(current_module);
    let mut parents = tcx.hir().parent_iter(current_module_hir_id);
    let mut base_module = current_module;
    while segments.next_if(|segment| segment == SUPER).is_some() {
        if let Some((parent, _)) = parents.next() {
            debug!("parent: {parent:?}");
            base_module = parent.owner.def_id;
        } else {
            return Err(ResolveError::ExtraSuper);
        }
    }
    debug!("base: {base_module:?}");
    Ok(Path { base: base_module.to_def_id(), segments: segments.collect() })
}

/// Resolves an external crate name.
fn resolve_external(tcx: TyCtxt, name: &str) -> Option<DefId> {
    debug!(?name, "resolve_external");
    tcx.used_crates(()).iter().find_map(|crate_num| {
        let crate_name = tcx.crate_name(*crate_num);
        if crate_name.as_str() == name {
            Some(DefId { index: CRATE_DEF_INDEX, krate: *crate_num })
        } else {
            None
        }
    })
}

/// Resolves a path relative to a foreign module.
fn resolve_in_foreign_module(tcx: TyCtxt, foreign_mod: DefId, name: &str) -> Option<DefId> {
    debug!(?name, ?foreign_mod, "resolve_in_foreign_module");
    tcx.module_children(foreign_mod)
        .iter()
        .find_map(|item| if item.ident.as_str() == name { item.res.opt_def_id() } else { None })
}

/// Generates a more friendly string representation of a def_id including kind and name.
/// (the default representation for the crate root is the empty string).
fn description(tcx: TyCtxt, def_id: DefId) -> String {
    let def_kind = tcx.def_kind(def_id);
    let kind_name = def_kind.descr(def_id);
    if def_id.is_crate_root() {
        format!("{kind_name} `{}`", tcx.crate_name(LOCAL_CRATE))
    } else {
        format!("{kind_name} `{}`", tcx.def_path_str(def_id))
    }
}

/// The possible result of trying to resolve the name relative to a local module.
enum RelativeResolution {
    /// Return the item that user requested.
    Found(DefId),
    /// Return all globs that may define the item requested.
    Globs(Vec<Res>),
}

/// Resolves a path relative to a local module.
fn resolve_relative(tcx: TyCtxt, current_module: LocalModDefId, name: &str) -> RelativeResolution {
    debug!(?name, ?current_module, "resolve_relative");

    let mut glob_imports = vec![];
    let result = tcx.hir().module_items(current_module).find_map(|item_id| {
        let item = tcx.hir().item(item_id);
        if item.ident.as_str() == name {
            match item.kind {
                ItemKind::Use(use_path, UseKind::Single) => use_path.res[0].opt_def_id(),
                ItemKind::ExternCrate(orig_name) => resolve_external(
                    tcx,
                    orig_name.as_ref().map(|sym| sym.as_str()).unwrap_or(name),
                ),
                _ => Some(item.owner_id.def_id.to_def_id()),
            }
        } else {
            if let ItemKind::Use(use_path, UseKind::Glob) = item.kind {
                // Do not immediately try to resolve the path using this glob,
                // since paths resolved via non-globs take precedence.
                glob_imports.push(use_path.res[0]);
            }
            None
        }
    });
    result.map_or(RelativeResolution::Globs(glob_imports), RelativeResolution::Found)
}

/// Resolves a path relative to a local or foreign module.
/// For local modules, if no module item matches the name we also have to traverse the list of glob
/// imports. For foreign modules, that list should've been flatten already.
fn resolve_in_module<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: DefId,
    name: &str,
) -> Result<DefId, ResolveError<'tcx>> {
    match current_module.as_local() {
        None => resolve_in_foreign_module(tcx, current_module, name).ok_or_else(|| {
            ResolveError::MissingItem { tcx, base: current_module, unresolved: name.to_string() }
        }),
        Some(local_id) => {
            let result = resolve_relative(tcx, LocalModDefId::new_unchecked(local_id), name);
            match result {
                RelativeResolution::Found(def_id) => Ok(def_id),
                RelativeResolution::Globs(globs) => {
                    resolve_in_glob_uses(tcx, local_id, globs, name)
                }
            }
        }
    }
}

/// Resolves a path by exploring glob use statements.
/// Note that there could be loops in glob use statements, so we need to track modules that have
/// been visited.
fn resolve_in_glob_uses<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    mut glob_resolutions: Vec<Res>,
    name: &str,
) -> Result<DefId, ResolveError<'tcx>> {
    let mut visited = HashSet::<Res>::default();
    let mut matches = vec![];
    while let Some(res) = glob_resolutions.pop() {
        if !visited.contains(&res) {
            visited.insert(res);
            let result = resolve_in_glob_use(tcx, &res, name);
            match result {
                RelativeResolution::Found(def_id) => matches.push(def_id),
                RelativeResolution::Globs(mut other_globs) => {
                    glob_resolutions.append(&mut other_globs)
                }
            }
        }
    }
    match matches.len() {
        0 => Err(ResolveError::MissingItem {
            tcx,
            base: current_module.to_def_id(),
            unresolved: name.to_string(),
        }),
        1 => Ok(matches.pop().unwrap()),
        _ => Err(ResolveError::AmbiguousGlob {
            tcx,
            base: current_module.to_def_id(),
            name: name.to_string(),
            candidates: matches,
        }),
    }
}

/// Resolves a path by exploring a glob use statement.
fn resolve_in_glob_use(tcx: TyCtxt, res: &Res, name: &str) -> RelativeResolution {
    if let Res::Def(DefKind::Mod, def_id) = res {
        if let Some(local_id) = def_id.as_local() {
            resolve_relative(tcx, LocalModDefId::new_unchecked(local_id), name)
        } else {
            resolve_in_foreign_module(tcx, *def_id, name)
                .map_or(RelativeResolution::Globs(vec![]), RelativeResolution::Found)
        }
    } else {
        // This shouldn't happen. Only module imports can use globs.
        RelativeResolution::Globs(vec![])
    }
}

/// Resolves a method in a type. It currently does not resolve trait methods
/// (see <https://github.com/model-checking/kani/issues/1997>).
fn resolve_in_type<'tcx>(
    tcx: TyCtxt<'tcx>,
    type_id: DefId,
    name: &str,
) -> Result<DefId, ResolveError<'tcx>> {
    debug!(?name, ?type_id, "resolve_in_type");
    let missing_item_err =
        || ResolveError::MissingItem { tcx, base: type_id, unresolved: name.to_string() };
    // Try the inherent `impl` blocks (i.e., non-trait `impl`s).
    tcx.inherent_impls(type_id)
        .map_err(|_| missing_item_err())?
        .iter()
        .flat_map(|impl_id| tcx.associated_item_def_ids(impl_id))
        .cloned()
        .find(|item| {
            let item_path = tcx.def_path_str(*item);
            let last = item_path.split("::").last().unwrap();
            last == name
        })
        .ok_or_else(missing_item_err)
}
