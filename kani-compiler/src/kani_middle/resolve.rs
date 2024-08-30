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

use crate::kani_middle::stable_fn_def;
use quote::ToTokens;
use rustc_errors::ErrorGuaranteed;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId, LocalModDefId, CRATE_DEF_INDEX, LOCAL_CRATE};
use rustc_hir::{ItemKind, UseKind};
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::ty::{FnDef, RigidTy, Ty, TyKind};
use stable_mir::CrateDef;
use std::collections::HashSet;
use std::fmt;
use std::iter::Peekable;
use std::str::FromStr;
use strum_macros::{EnumString, IntoStaticStr};
use syn::{Ident, PathSegment, Type, TypePath};
use tracing::debug;

#[derive(Copy, Clone, Debug, Eq, PartialEq, IntoStaticStr, EnumString)]
#[strum(serialize_all = "lowercase")]
enum PrimitiveIdent {
    Bool,
    Char,
    F16,
    F32,
    F64,
    F128,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    Str,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

macro_rules! validate_kind {
    ($tcx:ident, $id:ident, $expected:literal, $kind:pat) => {{
        let def_kind = $tcx.def_kind($id);
        if matches!(def_kind, $kind) {
            Ok($id)
        } else {
            Err(ResolveError::UnexpectedType { $tcx, item: $id, expected: $expected })
        }
    }};
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FnResolution {
    Fn(FnDef),
    FnImpl { def: FnDef, ty: Ty },
}

/// Resolve a path to a function / method.
///
/// The path can either be a simple path or a qualified path.
pub fn resolve_fn_path<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    path: &TypePath,
) -> Result<FnResolution, ResolveError<'tcx>> {
    match (&path.qself, &path.path.leading_colon) {
        (Some(qself), Some(_)) => {
            // Qualified path that does not define a trait.
            resolve_ty(tcx, current_module, &qself.ty)?;
            Err(ResolveError::UnsupportedPath { kind: "qualified bare function paths" })
        }
        (Some(qself), None) => {
            let ty = resolve_ty(tcx, current_module, &qself.ty)?;
            let def_id = resolve_path(tcx, current_module, &path.path)?;
            validate_kind!(tcx, def_id, "function / method", DefKind::Fn | DefKind::AssocFn)?;
            Ok(FnResolution::FnImpl { def: stable_fn_def(tcx, def_id).unwrap(), ty })
        }
        (None, _) => {
            // Simple path
            let def_id = resolve_path(tcx, current_module, &path.path)?;
            validate_kind!(tcx, def_id, "function / method", DefKind::Fn | DefKind::AssocFn)?;
            Ok(FnResolution::Fn(stable_fn_def(tcx, def_id).unwrap()))
        }
    }
}

/// Attempts to resolve a simple path (in the form of a string) to a function / method `DefId`.
///
/// Use `[resolve_fn_path]` if you want to handle qualified paths and simple paths
pub fn resolve_fn<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    path_str: &str,
) -> Result<DefId, ResolveError<'tcx>> {
    let path = syn::parse_str(path_str).map_err(|err| ResolveError::InvalidPath {
        msg: format!("Expected a path, but found `{path_str}`. {err}"),
    })?;
    let result = resolve_fn_path(tcx, current_module, &path)?;
    if let FnResolution::Fn(def) = result {
        Ok(rustc_internal::internal(tcx, def.def_id()))
    } else {
        Err(ResolveError::UnsupportedPath { kind: "qualified paths" })
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

/// Attempts to resolve a type.
pub fn resolve_ty<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    typ: &syn::Type,
) -> Result<Ty, ResolveError<'tcx>> {
    debug!(?typ, ?current_module, "resolve_ty");
    let unsupported = |kind: &'static str| Err(ResolveError::UnsupportedPath { kind });
    let invalid = |kind: &'static str| {
        Err(ResolveError::InvalidPath {
            msg: format!("Expected a type, but found {kind} `{}`", typ.to_token_stream()),
        })
    };
    #[warn(non_exhaustive_omitted_patterns)]
    match typ {
        Type::Path(path) if path.qself.is_none() => {
            let def_id = resolve_path(tcx, current_module, &path.path)?;
            validate_kind!(tcx, def_id, "type", DefKind::Struct | DefKind::Union | DefKind::Enum)?;
            Ok(rustc_internal::stable(tcx.type_of(def_id)).value)
        }
        Type::Path(_) => unsupported("qualified paths"),
        Type::Array(_)
        | Type::BareFn(_)
        | Type::Macro(_)
        | Type::Never(_)
        | Type::Paren(_)
        | Type::Ptr(_)
        | Type::Reference(_)
        | Type::Slice(_)
        | Type::Tuple(_) => unsupported("path including primitive types"),
        Type::Verbatim(_) => unsupported("unknown paths"),
        Type::Group(_) => invalid("group paths"),
        Type::ImplTrait(_) => invalid("trait impl paths"),
        Type::Infer(_) => invalid("inferred paths"),
        Type::TraitObject(_) => invalid("trait object paths"),
        _ => {
            unreachable!()
        }
    }
}

/// Checks if a Path segment represents a primitive
fn is_primitive(ident: &Ident) -> bool {
    let token = ident.to_string();
    let Ok(typ) = syn::parse_str(&token) else { return false };
    #[warn(non_exhaustive_omitted_patterns)]
    match typ {
        Type::Array(_)
        | Type::Ptr(_)
        | Type::Reference(_)
        | Type::Slice(_)
        | Type::Never(_)
        | Type::Tuple(_) => true,
        Type::Path(_) => PrimitiveIdent::from_str(&token).is_ok(),
        Type::BareFn(_)
        | Type::Group(_)
        | Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Paren(_)
        | Type::TraitObject(_)
        | Type::Verbatim(_) => false,
        _ => {
            unreachable!()
        }
    }
}

/// Attempts to resolve a simple path (in the form of a string) to a `DefId`.
/// The current module is provided as an argument in order to resolve relative
/// paths.
fn resolve_path<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    path: &syn::Path,
) -> Result<DefId, ResolveError<'tcx>> {
    debug!(?path, "resolve_path");
    let _span = tracing::span!(tracing::Level::DEBUG, "path_resolution").entered();

    let path = resolve_prefix(tcx, current_module, path)?;
    path.segments.into_iter().try_fold(path.base, |base, segment| {
        let name = segment.ident.to_string();
        let def_kind = tcx.def_kind(base);
        let next_item = match def_kind {
            DefKind::ForeignMod | DefKind::Mod => resolve_in_module(tcx, base, &name),
            DefKind::Struct | DefKind::Enum | DefKind::Union => resolve_in_type(tcx, base, &name),
            DefKind::Trait => resolve_in_trait(tcx, base, &name),
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
    /// Error triggered when the identifier is not currently supported.
    UnsupportedPath { kind: &'static str },
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
            ResolveError::UnsupportedPath { kind } => {
                write!(f, "Kani currently cannot resolve {kind}")
            }
        }
    }
}

/// The segments of a path.
type Segments = Vec<PathSegment>;

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
/// Identifier for the current module.
const SELF: &str = "self";
/// Identifier for the parent of the current module.
const SUPER: &str = "super";

/// Takes a string representation of a path and turns it into a `Path` data
/// structure, resolving prefix qualifiers (like `crate`, `self`, etc.) along the way.
fn resolve_prefix<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    path: &syn::Path,
) -> Result<Path, ResolveError<'tcx>> {
    debug!(?path, ?current_module, "resolve_prefix");

    // Split the string into segments separated by `::`. Trim the whitespace
    // since path strings generated from macros sometimes add spaces around
    // `::`.
    let mut segments = path.segments.iter();

    // Resolve qualifiers `crate`, initial `::`, and `self`. The qualifier
    // `self` may be followed be `super` (handled below).
    match (path.leading_colon, segments.next()) {
        (Some(_), Some(segment)) => {
            // Skip root and get the external crate from the name that follows `::`.
            let next_name = segment.ident.to_string();
            let result = resolve_external(tcx, &next_name);
            if let Some(def_id) = result {
                Ok(Path { base: def_id, segments: segments.cloned().collect() })
            } else {
                Err(ResolveError::MissingItem {
                    tcx,
                    base: current_module.to_def_id(),
                    unresolved: next_name,
                })
            }
        }
        (Some(_), None) => {
            Err(ResolveError::InvalidPath { msg: "expected identifier after `::`".to_string() })
        }
        (None, Some(segment)) if segment.ident == CRATE => {
            // Find the module at the root of the crate.
            let current_module_hir_id = tcx.local_def_id_to_hir_id(current_module);
            let crate_root = match tcx.hir().parent_iter(current_module_hir_id).last() {
                None => current_module,
                Some((hir_id, _)) => hir_id.owner.def_id,
            };
            Ok(Path { base: crate_root.to_def_id(), segments: segments.cloned().collect() })
        }
        (None, Some(segment)) if segment.ident == SELF => {
            resolve_super(tcx, current_module, segments.peekable())
        }
        (None, Some(segment)) if segment.ident == SUPER => {
            resolve_super(tcx, current_module, path.segments.iter().peekable())
        }
        (None, Some(segment)) if is_primitive(&segment.ident) => {
            Err(ResolveError::UnsupportedPath { kind: "path including primitive types" })
        }
        (None, Some(segment)) => {
            // No special key word was used. Try local first otherwise try external name.
            let next_name = segment.ident.to_string();
            let def_id =
                resolve_in_module(tcx, current_module.to_def_id(), &next_name).or_else(|err| {
                    if matches!(err, ResolveError::MissingItem { .. }) {
                        // Only try external if we couldn't find anything.
                        resolve_external(tcx, &next_name).ok_or(err)
                    } else {
                        Err(err)
                    }
                })?;
            Ok(Path { base: def_id, segments: segments.cloned().collect() })
        }
        _ => {
            unreachable!("Empty path: `{path:?}`")
        }
    }
}

/// Pop up the module stack until we account for all the `super` prefixes.
/// This method will error out if it tries to backtrace from the root crate.
fn resolve_super<'tcx, 'a, I>(
    tcx: TyCtxt,
    current_module: LocalDefId,
    mut segments: Peekable<I>,
) -> Result<Path, ResolveError<'tcx>>
where
    I: Iterator<Item = &'a PathSegment>,
{
    let current_module_hir_id = tcx.local_def_id_to_hir_id(current_module);
    let mut parents = tcx.hir().parent_iter(current_module_hir_id);
    let mut base_module = current_module;
    while segments.next_if(|segment| segment.ident == SUPER).is_some() {
        if let Some((parent, _)) = parents.next() {
            debug!("parent: {parent:?}");
            base_module = parent.owner.def_id;
        } else {
            return Err(ResolveError::ExtraSuper);
        }
    }
    debug!("base: {base_module:?}");
    Ok(Path { base: base_module.to_def_id(), segments: segments.cloned().collect() })
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

/// Resolves a function in a type.
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

/// Resolves a function in a trait.
fn resolve_in_trait<'tcx>(
    tcx: TyCtxt<'tcx>,
    trait_id: DefId,
    name: &str,
) -> Result<DefId, ResolveError<'tcx>> {
    debug!(?name, ?trait_id, "resolve_in_trait");
    let missing_item_err =
        || ResolveError::MissingItem { tcx, base: trait_id, unresolved: name.to_string() };
    let trait_def = tcx.trait_def(trait_id);
    // Try the inherent `impl` blocks (i.e., non-trait `impl`s).
    tcx.associated_item_def_ids(trait_def.def_id)
        .iter()
        .copied()
        .find(|item| {
            let item_path = tcx.def_path_str(*item);
            let last = item_path.split("::").last().unwrap();
            last == name
        })
        .ok_or_else(missing_item_err)
}
