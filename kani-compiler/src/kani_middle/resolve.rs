// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for resolving strings to `DefId`s.

use std::collections::VecDeque;

use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId, CRATE_DEF_INDEX};
use rustc_hir::ItemKind;
use rustc_middle::ty::TyCtxt;

/// Attempts to resolve a path (in the form of a string) to a `DefId`. The
/// current module is provided as an argument in order to resolve relative
/// paths.
pub fn resolve_path(tcx: TyCtxt, current_module: LocalDefId, path_str: &str) -> Option<DefId> {
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

/// A path consisting of a starting point and a bunch of segments.
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
    const CRATE: &str = "crate";
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
    let mut may_be_external_path = true;
    let first = segments[0].as_str();
    if first == ROOT {
        segments.pop_front();
        return Some(Path::new(Base::ExternPrelude, segments));
    } else if first == CRATE {
        segments.pop_front();
        // Find the module at the root of the crate.
        let current_module_hir_id = tcx.hir().local_def_id_to_hir_id(current_module);
        let crate_root = match tcx.hir().parent_iter(current_module_hir_id).last() {
            None => current_module,
            Some((hir_id, _)) => tcx.hir().local_def_id(hir_id),
        };
        return Some(Path::new(
            Base::LocalModule { id: crate_root, may_be_external_path: false },
            segments,
        ));
    } else if first == SELF {
        segments.pop_front();
        may_be_external_path = false;
    }

    // Pop up the module stack until we account for all the `super` prefixes.
    let current_module_hir_id = tcx.hir().local_def_id_to_hir_id(current_module);
    let mut parents = tcx.hir().parent_iter(current_module_hir_id);
    let mut base_module = current_module;
    while segments.front().map(String::as_str) == Some(SUPER) {
        segments.pop_front();
        base_module = tcx.hir().local_def_id(parents.next()?.0);
        may_be_external_path = false;
    }

    Some(Path::new(Base::LocalModule { id: base_module, may_be_external_path }, segments))
}

/// Resolves an external path.
fn resolve_external(tcx: TyCtxt, mut segments: Segments) -> Option<DefId> {
    let first = segments.pop_front()?;
    for crate_num in tcx.crates(()) {
        let crate_name = tcx.crate_name(*crate_num);
        if crate_name.as_str() == first {
            let crate_def_id = DefId { index: CRATE_DEF_INDEX, krate: *crate_num };
            return resolve_in_foreign_module(tcx, crate_def_id, segments);
        }
    }
    None
}

/// Resolves a path relative to a foreign module.
fn resolve_in_foreign_module(
    tcx: TyCtxt,
    foreign_mod: DefId,
    mut segments: Segments,
) -> Option<DefId> {
    let first = segments.front()?;
    for child in tcx.module_children(foreign_mod) {
        match child.res {
            Res::Def(DefKind::Fn, def_id) => {
                if first == child.ident.as_str() && segments.len() == 1 {
                    return Some(def_id);
                }
            }
            Res::Def(DefKind::Mod, inner_mod_id) => {
                if first == child.ident.as_str() {
                    segments.pop_front();
                    return resolve_in_foreign_module(tcx, inner_mod_id, segments);
                }
            }
            Res::Def(DefKind::Struct, type_id) | Res::Def(DefKind::Enum, type_id) => {
                if first == child.ident.as_str() && segments.len() == 2 {
                    let maybe_resolved = resolve_in_inherent_impls(tcx, type_id, &segments[1]);
                    if maybe_resolved.is_some() {
                        return maybe_resolved;
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Resolves a path relative to a local module.
fn resolve_relative(
    tcx: TyCtxt,
    current_module: LocalDefId,
    mut segments: Segments,
) -> Option<DefId> {
    let first = segments.front()?;
    for item_id in tcx.hir().module_items(current_module) {
        let item = tcx.hir().item(item_id);
        let def_id = item.owner_id.def_id.to_def_id();
        match item.kind {
            ItemKind::Fn(..) => {
                if first == item.ident.as_str() && segments.len() == 1 {
                    return Some(def_id);
                }
            }
            ItemKind::Mod(..) => {
                if first == item.ident.as_str() {
                    segments.pop_front();
                    return resolve_relative(tcx, def_id.expect_local(), segments);
                }
            }
            ItemKind::Enum(..) | ItemKind::Struct(..) => {
                if first == item.ident.as_str() && segments.len() == 2 {
                    let maybe_resolved = resolve_in_inherent_impls(tcx, def_id, &segments[1]);
                    if maybe_resolved.is_some() {
                        return maybe_resolved;
                    }
                }
            }
            _ => (),
        }
    }
    None
}

/// Resolves a name in an `impl` block.
fn resolve_in_impl(tcx: TyCtxt, impl_id: DefId, name: &str) -> Option<DefId> {
    for assoc_item in tcx.associated_item_def_ids(impl_id) {
        let item_path = tcx.def_path_str(*assoc_item);
        let last = item_path.split("::").last().unwrap();
        if last == name {
            return Some(*assoc_item);
        }
    }
    None
}

/// Resolves a name in the inherent `impl` blocks of a type (i.e., non-trait
/// `impl`s).
fn resolve_in_inherent_impls(tcx: TyCtxt, type_id: DefId, name: &str) -> Option<DefId> {
    for impl_ in tcx.inherent_impls(type_id) {
        let maybe_resolved = resolve_in_impl(tcx, *impl_, name);
        if maybe_resolved.is_some() {
            return maybe_resolved;
        }
    }
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
