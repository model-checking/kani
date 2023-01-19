// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for resolving strings representing simple paths to
//! `DefId`s for functions and methods. For the definition of a simple path, see
//! <https://doc.rust-lang.org/reference/paths.html#simple-paths>.
//!
//! TODO: Extend this logic to support resolving qualified paths.
//! <https://github.com/model-checking/kani/issues/1997>

use std::collections::VecDeque;

use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId, CRATE_DEF_INDEX, LOCAL_CRATE};
use rustc_hir::{ItemKind, UseKind};
use rustc_middle::ty::TyCtxt;

/// Attempts to resolve a simple path (in the form of a string) to a `DefId`.
/// The current module is provided as an argument in order to resolve relative
/// paths.
///
/// TODO: Extend this implementation to handle qualified paths and simple paths
/// corresponding to trait methods.
/// <https://github.com/model-checking/kani/issues/1997>
pub fn resolve_path(tcx: TyCtxt, current_module: LocalDefId, path_str: &str) -> Option<DefId> {
    let span = tracing::span!(tracing::Level::DEBUG, "path_resolution");
    let _enter = span.enter();

    let path = to_path(tcx, current_module, path_str)?;
    match &path.base {
        Base::ExternPrelude => resolve_external(tcx, path.segments),
        Base::LocalModule { id, may_be_external_path } => {
            // Try to resolve it as a relative path first; if this fails and the
            // path might be external (it wasn't qualified with `self`, etc.)
            // and the current module does not have a submodule with the same
            // first segment, try resolving it as an external path.
            resolve_relative(tcx, *id, path.segments.clone()).or_else(|| {
                if *may_be_external_path
                    && !has_submodule_with_name(tcx, current_module, path.segments.front()?)
                {
                    resolve_external(tcx, path.segments)
                } else {
                    None
                }
            })
        }
    }
}

/// The segments of a path.
type Segments = VecDeque<String>;

/// Generates the string representation of the path composed of these segments
/// (it is inefficient, but we use it only in debugging output).
fn segments_to_string(segments: &Segments) -> String {
    segments.iter().cloned().collect::<Vec<_>>().join("::")
}

/// The "starting point" for a path.
#[derive(Debug)]
enum Base {
    /// Indicates an external path.
    ExternPrelude,
    /// Indicates a path that may be local (and must be local if
    /// `may_be_external_path` is false) and should be resolved relative to the
    /// module identified by `id`.
    LocalModule { id: LocalDefId, may_be_external_path: bool },
}

/// A path consisting of a starting point and a bunch of segments. If `base`
/// matches `Base::LocalModule { id: _, may_be_external_path : true }`, then
/// `segments` cannot be empty.
#[derive(Debug)]
struct Path {
    base: Base,
    segments: Segments,
}

impl Path {
    fn new(base: Base, segments: Segments) -> Self {
        Path { base, segments }
    }
}

/// Takes a string representation of a path and turns it into a `Path` data
/// structure, resolving qualifiers (like `crate`, etc.) along the way.
fn to_path(tcx: TyCtxt, current_module: LocalDefId, name: &str) -> Option<Path> {
    tracing::debug!("Normalizing path `{name}`");

    const CRATE: &str = "crate";
    // rustc represents initial `::` as `{{root}}`.
    const ROOT: &str = "{{root}}";
    const SELF: &str = "self";
    const SUPER: &str = "super";

    // Split the string into segments separated by `::`.
    let mut segments: Segments = name.split("::").map(str::to_string).collect();
    if segments.is_empty() {
        return Some(Path::new(
            Base::LocalModule { id: current_module, may_be_external_path: false },
            segments,
        ));
    }

    // Resolve qualifiers `crate`, initial `::`, and `self`. The qualifier
    // `self` may be followed be `super` (handled below).
    let first = segments[0].as_str();
    let may_be_external_path = !matches!(first, CRATE | SELF | SUPER);
    match first {
        ROOT => {
            segments.pop_front();
            return Some(Path::new(Base::ExternPrelude, segments));
        }
        CRATE => {
            segments.pop_front();
            // Find the module at the root of the crate.
            let current_module_hir_id = tcx.hir().local_def_id_to_hir_id(current_module);
            let crate_root = match tcx.hir().parent_iter(current_module_hir_id).last() {
                None => current_module,
                Some((hir_id, _)) => tcx.hir().local_def_id(hir_id),
            };
            return Some(Path::new(
                Base::LocalModule { id: crate_root, may_be_external_path },
                segments,
            ));
        }
        SELF => {
            segments.pop_front();
        }
        _ => (),
    }

    // Pop up the module stack until we account for all the `super` prefixes.
    let current_module_hir_id = tcx.hir().local_def_id_to_hir_id(current_module);
    let mut parents = tcx.hir().parent_iter(current_module_hir_id);
    let mut base_module = current_module;
    while segments.front().map(String::as_str) == Some(SUPER) {
        segments.pop_front();
        let parent = parents.next().map(|p| p.0).or_else(|| {
            tracing::debug!("Unable to normalize path `{name}`: too many `super` qualifiers");
            None
        })?;
        base_module = tcx.hir().local_def_id(parent);
    }

    Some(Path::new(Base::LocalModule { id: base_module, may_be_external_path }, segments))
}

/// Resolves an external path.
fn resolve_external(tcx: TyCtxt, mut segments: Segments) -> Option<DefId> {
    tracing::debug!("Resolving `{}` in the external prelude", segments_to_string(&segments));
    let first = segments.pop_front().or_else(|| {
        tracing::debug!("Unable to resolve the empty path");
        None
    })?;
    for crate_num in tcx.crates(()) {
        let crate_name = tcx.crate_name(*crate_num);
        if crate_name.as_str() == first {
            let crate_def_id = DefId { index: CRATE_DEF_INDEX, krate: *crate_num };
            return resolve_in_foreign_module(tcx, crate_def_id, segments);
        }
    }
    tracing::debug!("Unable to resolve `{first}` as an external crate");
    None
}

/// Resolves a path relative to a foreign module.
fn resolve_in_foreign_module(
    tcx: TyCtxt,
    foreign_mod: DefId,
    mut segments: Segments,
) -> Option<DefId> {
    tracing::debug!(
        "Resolving `{}` in foreign module `{}`",
        segments_to_string(&segments),
        tcx.def_path_str(foreign_mod)
    );

    let first = segments.front().or_else(|| {
        tracing::debug!("Unable to resolve the empty path");
        None
    })?;
    for child in tcx.module_children(foreign_mod) {
        match child.res {
            Res::Def(DefKind::Fn, def_id) => {
                if first == child.ident.as_str() && segments.len() == 1 {
                    tracing::debug!(
                        "Resolved `{first}` as a function in foreign module `{}`",
                        tcx.def_path_str(foreign_mod)
                    );
                    return Some(def_id);
                }
            }
            Res::Def(DefKind::Mod, inner_mod_id) => {
                if first == child.ident.as_str() {
                    segments.pop_front();
                    return resolve_in_foreign_module(tcx, inner_mod_id, segments);
                }
            }
            Res::Def(DefKind::Struct | DefKind::Enum | DefKind::Union, type_id) => {
                if first == child.ident.as_str() && segments.len() == 2 {
                    return resolve_in_type(tcx, type_id, &segments[1]);
                }
            }
            _ => {}
        }
    }

    tracing::debug!(
        "Unable to resolve `{first}` as an item in foreign module `{}`",
        tcx.def_path_str(foreign_mod)
    );
    None
}

/// Generates a more friendly string representation of a local module's name
/// (the default representation for the crate root is the empty string).
fn module_to_string(tcx: TyCtxt, current_module: LocalDefId) -> String {
    let def_id = current_module.to_def_id();
    if def_id.is_crate_root() {
        format!("module `{}`", tcx.crate_name(LOCAL_CRATE))
    } else {
        format!("module `{}`", tcx.def_path_str(def_id))
    }
}

/// Resolves a path relative to a local module.
fn resolve_relative(
    tcx: TyCtxt,
    current_module: LocalDefId,
    mut segments: Segments,
) -> Option<DefId> {
    tracing::debug!(
        "Resolving `{}` in local {}",
        segments_to_string(&segments),
        module_to_string(tcx, current_module)
    );

    let first = segments.front().or_else(|| {
        tracing::debug!("Unable to resolve the empty path");
        None
    })?;
    let mut glob_imports = Vec::new();
    for item_id in tcx.hir().module_items(current_module) {
        let item = tcx.hir().item(item_id);
        let def_id = item.owner_id.def_id.to_def_id();
        match item.kind {
            ItemKind::Fn(..) => {
                if first == item.ident.as_str() && segments.len() == 1 {
                    tracing::debug!(
                        "Resolved `{first}` as a function in local {}",
                        module_to_string(tcx, current_module)
                    );
                    return Some(def_id);
                }
            }
            ItemKind::Mod(..) => {
                if first == item.ident.as_str() {
                    segments.pop_front();
                    return resolve_relative(tcx, def_id.expect_local(), segments);
                }
            }
            ItemKind::Enum(..) | ItemKind::Struct(..) | ItemKind::Union(..) => {
                if first == item.ident.as_str() && segments.len() == 2 {
                    return resolve_in_type(tcx, def_id, &segments[1]);
                }
            }
            ItemKind::Use(use_path, UseKind::Single) => {
                if first == item.ident.as_str() {
                    segments.pop_front();
                    return resolve_in_use(tcx, use_path, segments);
                }
            }
            ItemKind::Use(use_path, UseKind::Glob) => {
                // Do not immediately try to resolve the path using this glob,
                // since paths resolved via non-globs take precedence.
                glob_imports.push(use_path);
            }
            ItemKind::ExternCrate(orig_name_opt) => {
                if first == item.ident.as_str() {
                    if let Some(orig_name) = orig_name_opt {
                        segments[0] = orig_name.to_string();
                    }
                    return resolve_external(tcx, segments);
                }
            }
            _ => (),
        }
    }

    resolve_in_glob_uses(tcx, current_module, glob_imports, &segments).or_else(|| {
        tracing::debug!(
            "Unable to resolve `{first}` as an item in local {}",
            module_to_string(tcx, current_module)
        );
        None
    })
}

/// Resolves a path relative to a local or foreign module.
fn resolve_in_module(tcx: TyCtxt, current_module: DefId, segments: Segments) -> Option<DefId> {
    match current_module.as_local() {
        None => resolve_in_foreign_module(tcx, current_module, segments),
        Some(local_id) => resolve_relative(tcx, local_id, segments),
    }
}

/// Resolves a path by exploring a non-glob use statement.
fn resolve_in_use(tcx: TyCtxt, use_path: &rustc_hir::Path, segments: Segments) -> Option<DefId> {
    if let Res::Def(def_kind, def_id) = use_path.res {
        tracing::debug!(
            "Resolving `{}` via `use` import of `{}`",
            segments_to_string(&segments),
            tcx.def_path_str(def_id)
        );
        match def_kind {
            DefKind::Fn => {
                if segments.is_empty() {
                    tracing::debug!(
                        "Resolved `{}` to a function via `use` import of `{}`",
                        segments_to_string(&segments),
                        tcx.def_path_str(def_id)
                    );
                    return Some(def_id);
                }
            }
            DefKind::Mod => return resolve_in_module(tcx, def_id, segments),
            DefKind::Struct | DefKind::Enum | DefKind::Union => {
                if segments.len() == 1 {
                    return resolve_in_type(tcx, def_id, &segments[0]);
                }
            }
            _ => (),
        }
    }
    tracing::debug!("Unable to resolve `{}` via `use` import", segments_to_string(&segments));
    None
}

/// Resolves a path by exploring glob use statements.
fn resolve_in_glob_uses(
    tcx: TyCtxt,
    current_module: LocalDefId,
    glob_imports: Vec<&rustc_hir::Path>,
    segments: &Segments,
) -> Option<DefId> {
    let glob_resolves = glob_imports
        .iter()
        .filter_map(|use_path| {
            let span = tracing::span!(tracing::Level::DEBUG, "glob_resolution");
            let _enter = span.enter();
            resolve_in_glob_use(tcx, use_path, segments.clone())
        })
        .collect::<Vec<_>>();
    if glob_resolves.len() == 1 {
        return glob_resolves.first().copied();
    }
    if glob_resolves.len() > 1 {
        // Raise an error if it's ambiguous which glob import a function comes
        // from. rustc will also raise an error in this case if the ambiguous
        // function is present in code (and not just as an attribute argument).
        // TODO: We should make this consistent with error handling for other
        // cases (see <https://github.com/model-checking/kani/issues/2013>).
        let location = module_to_string(tcx, current_module);
        let mut msg = format!(
            "glob imports in local {location} make it impossible to \
            unambiguously resolve path; the possibilities are:"
        );
        for def_id in glob_resolves {
            msg.push_str("\n\t");
            msg.push_str(&tcx.def_path_str(def_id));
        }
        tcx.sess.err(msg);
    }
    None
}

/// Resolves a path by exploring a glob use statement.
fn resolve_in_glob_use(
    tcx: TyCtxt,
    use_path: &rustc_hir::Path,
    segments: Segments,
) -> Option<DefId> {
    if let Res::Def(DefKind::Mod, def_id) = use_path.res {
        resolve_in_module(tcx, def_id, segments)
    } else {
        None
    }
}

/// Resolves a method in a type. It currently does not resolve trait methods
/// (see <https://github.com/model-checking/kani/issues/1997>).
fn resolve_in_type(tcx: TyCtxt, type_id: DefId, name: &str) -> Option<DefId> {
    tracing::debug!("Resolving `{name}` in type `{}`", tcx.def_path_str(type_id));
    // Try the inherent `impl` blocks (i.e., non-trait `impl`s).
    for impl_ in tcx.inherent_impls(type_id) {
        let maybe_resolved = resolve_in_impl(tcx, *impl_, name);
        if maybe_resolved.is_some() {
            return maybe_resolved;
        }
    }
    tracing::debug!("Unable to resolve `{name}` in type `{}`", tcx.def_path_str(type_id));
    None
}

/// Resolves a name in an `impl` block.
fn resolve_in_impl(tcx: TyCtxt, impl_id: DefId, name: &str) -> Option<DefId> {
    tracing::debug!("Resolving `{name}` in impl block `{}`", tcx.def_path_str(impl_id));
    for assoc_item in tcx.associated_item_def_ids(impl_id) {
        let item_path = tcx.def_path_str(*assoc_item);
        let last = item_path.split("::").last().unwrap();
        if last == name {
            tracing::debug!("Resolved `{name}` in impl block `{}`", tcx.def_path_str(impl_id));
            return Some(*assoc_item);
        }
    }
    tracing::debug!("Unable to resolve `{name}` in impl block `{}`", tcx.def_path_str(impl_id));
    None
}

/// Does the current module have a (direct) submodule with the given name?
fn has_submodule_with_name(tcx: TyCtxt, current_module: LocalDefId, name: &str) -> bool {
    for item_id in tcx.hir().module_items(current_module) {
        let item = tcx.hir().item(item_id);
        if let ItemKind::Mod(..) = item.kind {
            if name == item.ident.as_str() {
                return true;
            }
        }
    }
    false
}
