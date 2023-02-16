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

use std::collections::HashSet;
use std::fmt;
use std::iter::Peekable;

use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId, CRATE_DEF_INDEX, LOCAL_CRATE};
use rustc_hir::{ItemKind, UseKind};
use rustc_middle::ty::TyCtxt;
use tracing::debug;

/// Attempts to resolve a simple path (in the form of a string) to a function / method `DefId`.
///
/// TODO: Extend this implementation to handle qualified paths and simple paths
/// corresponding to trait methods.
/// <https://github.com/model-checking/kani/issues/1997>
pub fn resolve_fn(
    tcx: TyCtxt,
    current_module: LocalDefId,
    path_str: &str,
) -> Result<DefId, ResolveError> {
    let result = resolve_path(tcx, current_module, path_str);
    match result {
        Ok(def_id) => {
            let def_kind = tcx.def_kind(def_id);
            if matches!(def_kind, DefKind::AssocFn | DefKind::Fn) {
                Ok(def_id)
            } else {
                let description = format!(
                    "expected function / method, found {} `{}`",
                    def_kind.descr(def_id),
                    tcx.def_path_str(def_id)
                );
                Err(ResolveError { msg: description })
            }
        }
        err => err,
    }
}

/// Attempts to resolve a simple path (in the form of a string) to a `DefId`.
/// The current module is provided as an argument in order to resolve relative
/// paths.
///
/// Note: This function was written to be generic, however, it has only been tested for functions.
fn resolve_path(
    tcx: TyCtxt,
    current_module: LocalDefId,
    path_str: &str,
) -> Result<DefId, ResolveError> {
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
                Err(ResolveError {
                    msg: format!(
                        "expected module, found {} `{}`",
                        def_kind.descr(base),
                        tcx.def_path_str(base)
                    ),
                })
            }
        };
        next_item
    })
}

/// Provide information about where the resolution failed.
/// Todo: Add error message.
pub struct ResolveError {
    pub msg: String,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl ResolveError {
    /// Generates a generic unable to find error.
    pub fn missing_item(tcx: TyCtxt, base: DefId, unresolved_name: &str) -> ResolveError {
        let def_desc = description(tcx, base);
        ResolveError { msg: format!("unable to find `{unresolved_name}` inside {def_desc}") }
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
fn resolve_prefix(
    tcx: TyCtxt,
    current_module: LocalDefId,
    name: &str,
) -> Result<Path, ResolveError> {
    debug!(?name, ?current_module, "resolve_prefix");

    // Split the string into segments separated by `::`.
    let mut segments = name.split("::").map(str::to_string).peekable();
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
                    Err(ResolveError::missing_item(tcx, current_module.to_def_id(), &next_name))
                }
            } else {
                Err(ResolveError { msg: "expected identifier after `::`".to_string() })
            }
        }
        CRATE => {
            segments.next();
            // Find the module at the root of the crate.
            let current_module_hir_id = tcx.hir().local_def_id_to_hir_id(current_module);
            let crate_root = match tcx.hir().parent_iter(current_module_hir_id).last() {
                None => current_module,
                Some((hir_id, _)) => tcx.hir().local_def_id(hir_id),
            };
            Ok(Path { base: crate_root.to_def_id(), segments: segments.collect() })
        }
        SELF => {
            segments.next();
            resolve_super(tcx, current_module, segments)
        }
        _ => {
            let path = resolve_super(tcx, current_module, segments)?;
            if !path.segments.is_empty() {
                let next_name = path.segments.first().unwrap();
                let def_id = resolve_in_module(tcx, path.base, &next_name)?;
                Ok(Path { base: def_id, segments: Vec::from(&path.segments[1..]) })
            } else {
                Ok(path)
            }
        }
    }
}

/// Pop up the module stack until we account for all the `super` prefixes.
/// This method will error out if it tries to backtrace from the root crate.
fn resolve_super<I>(
    tcx: TyCtxt,
    current_module: LocalDefId,
    mut segments: Peekable<I>,
) -> Result<Path, ResolveError>
where
    I: Iterator<Item = String>,
{
    let current_module_hir_id = tcx.hir().local_def_id_to_hir_id(current_module);
    let mut parents = tcx.hir().parent_iter(current_module_hir_id);
    let mut base_module = current_module;
    while segments.next_if(|segment| segment == SUPER).is_some() {
        if let Some((parent, _)) = parents.next() {
            debug!("parent: {parent:?}");
            base_module = tcx.hir().local_def_id(parent);
        } else {
            return Err(ResolveError {
                msg: "there are too many leading `super` keywords".to_string(),
            });
        }
    }
    debug!("base: {base_module:?}");
    Ok(Path { base: base_module.to_def_id(), segments: segments.collect() })
}

/// Resolves an external crate name.
fn resolve_external(tcx: TyCtxt, name: &str) -> Option<DefId> {
    debug!(?name, "resolve_external");
    tcx.crates(()).iter().find_map(|crate_num| {
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

/// Resolves a path relative to a local module.
fn resolve_relative(
    tcx: TyCtxt,
    current_module: LocalDefId,
    name: &str,
) -> Result<DefId, Vec<Res>> {
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
    result.ok_or(glob_imports)
}

/// Resolves a path relative to a local or foreign module.
/// For local modules, if no module item matches the name we also have to traverse the list of glob
/// imports. For foreign modules, that list should've been flatten already.
fn resolve_in_module(
    tcx: TyCtxt,
    current_module: DefId,
    name: &str,
) -> Result<DefId, ResolveError> {
    match current_module.as_local() {
        None => resolve_in_foreign_module(tcx, current_module, name)
            .ok_or_else(|| ResolveError::missing_item(tcx, current_module, name)),
        Some(local_id) => {
            let result = resolve_relative(tcx, local_id, name);
            result.or_else(|globs| resolve_in_glob_uses(tcx, local_id, globs, name))
        }
    }
}

/// Resolves a path by exploring glob use statements.
/// Note that there could be loops in glob use statements, so we need to track modules that have
/// been visited.
fn resolve_in_glob_uses(
    tcx: TyCtxt,
    current_module: LocalDefId,
    mut glob_resolutions: Vec<Res>,
    name: &str,
) -> Result<DefId, ResolveError> {
    let mut visited = HashSet::<Res>::default();
    let mut matches = vec![];
    while let Some(res) = glob_resolutions.pop() {
        if !visited.contains(&res) {
            visited.insert(res);
            let result = resolve_in_glob_use(tcx, &res, name);
            match result {
                Ok(def_id) => matches.push(def_id),
                Err(mut other_globs) => glob_resolutions.append(&mut other_globs),
            }
        }
    }
    match matches.as_slice() {
        [] => Err(ResolveError::missing_item(tcx, current_module.to_def_id(), name)),
        [def_id] => Ok(*def_id),
        ambiguous => {
            // Raise an error if it's ambiguous which glob import a function comes
            // from. rustc will also raise an error in this case if the ambiguous
            // function is present in code (and not just as an attribute argument).
            // TODO: We should make this consistent with error handling for other
            // cases (see <https://github.com/model-checking/kani/issues/2013>).
            let location = description(tcx, current_module.to_def_id());
            Err(ResolveError {
                msg: format!(
                    "`{name}` is ambiguous because of multiple glob imports in {location}. Found:\n{}",
                    ambiguous
                        .iter()
                        .map(|def_id| tcx.def_path_str(*def_id))
                        .intersperse("\n".to_string())
                        .collect::<String>()
                ),
            })
        }
    }
}

/// Resolves a path by exploring a glob use statement.
fn resolve_in_glob_use(tcx: TyCtxt, res: &Res, name: &str) -> Result<DefId, Vec<Res>> {
    if let Res::Def(DefKind::Mod, def_id) = res {
        if let Some(local_id) = def_id.as_local() {
            resolve_relative(tcx, local_id, name)
        } else {
            resolve_in_foreign_module(tcx, *def_id, name).ok_or(vec![])
        }
    } else {
        // This shouldn't happen. Only module imports can use globs.
        Err(vec![])
    }
}

/// Resolves a method in a type. It currently does not resolve trait methods
/// (see <https://github.com/model-checking/kani/issues/1997>).
fn resolve_in_type(tcx: TyCtxt, type_id: DefId, name: &str) -> Result<DefId, ResolveError> {
    debug!(?name, ?type_id, "resolve_in_type");
    // Try the inherent `impl` blocks (i.e., non-trait `impl`s).
    tcx.inherent_impls(type_id)
        .iter()
        .flat_map(|impl_id| tcx.associated_item_def_ids(impl_id))
        .cloned()
        .find(|item| {
            let item_path = tcx.def_path_str(*item);
            let last = item_path.split("::").last().unwrap();
            last == name
        })
        .ok_or_else(|| ResolveError::missing_item(tcx, type_id, name))
}
