//! Validates all used crates and extern libraries and loads their metadata

use crate::locator::{CrateError, CrateLocator, CratePaths};
use crate::rmeta::{CrateDep, CrateMetadata, CrateNumMap, CrateRoot, MetadataBlob};

use rustc_ast::expand::allocator::AllocatorKind;
use rustc_ast::{self as ast, *};
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_data_structures::svh::Svh;
use rustc_data_structures::sync::Lrc;
use rustc_expand::base::SyntaxExtension;
use rustc_hir::def_id::{CrateNum, LocalDefId, StableCrateId, LOCAL_CRATE};
use rustc_hir::definitions::Definitions;
use rustc_index::vec::IndexVec;
use rustc_middle::ty::TyCtxt;
use rustc_serialize::json::ToJson;
use rustc_session::config::{self, CrateType, ExternLocation};
use rustc_session::cstore::{CrateDepKind, CrateSource, ExternCrate};
use rustc_session::cstore::{ExternCrateSource, MetadataLoaderDyn};
use rustc_session::lint::{self, BuiltinLintDiagnostics, ExternDepSpec};
use rustc_session::output::validate_crate_name;
use rustc_session::search_paths::PathKind;
use rustc_session::Session;
use rustc_span::edition::Edition;
use rustc_span::symbol::{sym, Symbol};
use rustc_span::{Span, DUMMY_SP};
use rustc_target::spec::{PanicStrategy, TargetTriple};

use proc_macro::bridge::client::ProcMacro;
use std::collections::BTreeMap;
use std::ops::Fn;
use std::path::Path;
use std::{cmp, env};
use tracing::{debug, info};

#[derive(Clone)]
pub struct CStore {
    metas: IndexVec<CrateNum, Option<Lrc<CrateMetadata>>>,
    injected_panic_runtime: Option<CrateNum>,
    /// This crate needs an allocator and either provides it itself, or finds it in a dependency.
    /// If the above is true, then this field denotes the kind of the found allocator.
    allocator_kind: Option<AllocatorKind>,
    /// This crate has a `#[global_allocator]` item.
    has_global_allocator: bool,

    /// This map is used to verify we get no hash conflicts between
    /// `StableCrateId` values.
    pub(crate) stable_crate_ids: FxHashMap<StableCrateId, CrateNum>,

    /// Unused externs of the crate
    unused_externs: Vec<Symbol>,
}

impl std::fmt::Debug for CStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CStore").finish_non_exhaustive()
    }
}

pub struct CrateLoader<'a> {
    // Immutable configuration.
    sess: &'a Session,
    metadata_loader: Box<MetadataLoaderDyn>,
    local_crate_name: Symbol,
    // Mutable output.
    cstore: CStore,
    used_extern_options: FxHashSet<Symbol>,
}

pub enum LoadedMacro {
    MacroDef(ast::Item, Edition),
    ProcMacro(SyntaxExtension),
}

crate struct Library {
    pub source: CrateSource,
    pub metadata: MetadataBlob,
}

enum LoadResult {
    Previous(CrateNum),
    Loaded(Library),
}

/// A reference to `CrateMetadata` that can also give access to whole crate store when necessary.
#[derive(Clone, Copy)]
crate struct CrateMetadataRef<'a> {
    pub cdata: &'a CrateMetadata,
    pub cstore: &'a CStore,
}

impl std::ops::Deref for CrateMetadataRef<'_> {
    type Target = CrateMetadata;

    fn deref(&self) -> &Self::Target {
        self.cdata
    }
}

struct CrateDump<'a>(&'a CStore);

impl<'a> std::fmt::Debug for CrateDump<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(fmt, "resolved crates:")?;
        for (cnum, data) in self.0.iter_crate_data() {
            writeln!(fmt, "  name: {}", data.name())?;
            writeln!(fmt, "  cnum: {}", cnum)?;
            writeln!(fmt, "  hash: {}", data.hash())?;
            writeln!(fmt, "  reqd: {:?}", data.dep_kind())?;
            let CrateSource { dylib, rlib, rmeta } = data.source();
            if let Some(dylib) = dylib {
                writeln!(fmt, "  dylib: {}", dylib.0.display())?;
            }
            if let Some(rlib) = rlib {
                writeln!(fmt, "   rlib: {}", rlib.0.display())?;
            }
            if let Some(rmeta) = rmeta {
                writeln!(fmt, "   rmeta: {}", rmeta.0.display())?;
            }
        }
        Ok(())
    }
}

impl CStore {
    pub fn from_tcx(tcx: TyCtxt<'_>) -> &CStore {
        tcx.cstore_untracked()
            .as_any()
            .downcast_ref::<CStore>()
            .expect("`tcx.cstore` is not a `CStore`")
    }

    fn alloc_new_crate_num(&mut self) -> CrateNum {
        self.metas.push(None);
        CrateNum::new(self.metas.len() - 1)
    }

    crate fn get_crate_data(&self, cnum: CrateNum) -> CrateMetadataRef<'_> {
        let cdata = self.metas[cnum]
            .as_ref()
            .unwrap_or_else(|| panic!("Failed to get crate data for {:?}", cnum));
        CrateMetadataRef { cdata, cstore: self }
    }

    fn set_crate_data(&mut self, cnum: CrateNum, data: CrateMetadata) {
        assert!(self.metas[cnum].is_none(), "Overwriting crate metadata entry");
        self.metas[cnum] = Some(Lrc::new(data));
    }

    crate fn iter_crate_data(&self) -> impl Iterator<Item = (CrateNum, &CrateMetadata)> {
        self.metas
            .iter_enumerated()
            .filter_map(|(cnum, data)| data.as_ref().map(|data| (cnum, &**data)))
    }

    fn push_dependencies_in_postorder(&self, deps: &mut Vec<CrateNum>, cnum: CrateNum) {
        if !deps.contains(&cnum) {
            let data = self.get_crate_data(cnum);
            for &dep in data.dependencies().iter() {
                if dep != cnum {
                    self.push_dependencies_in_postorder(deps, dep);
                }
            }

            deps.push(cnum);
        }
    }

    crate fn crate_dependencies_in_postorder(&self, cnum: CrateNum) -> Vec<CrateNum> {
        let mut deps = Vec::new();
        if cnum == LOCAL_CRATE {
            for (cnum, _) in self.iter_crate_data() {
                self.push_dependencies_in_postorder(&mut deps, cnum);
            }
        } else {
            self.push_dependencies_in_postorder(&mut deps, cnum);
        }
        deps
    }

    fn crate_dependencies_in_reverse_postorder(&self, cnum: CrateNum) -> Vec<CrateNum> {
        let mut deps = self.crate_dependencies_in_postorder(cnum);
        deps.reverse();
        deps
    }

    crate fn injected_panic_runtime(&self) -> Option<CrateNum> {
        self.injected_panic_runtime
    }

    crate fn allocator_kind(&self) -> Option<AllocatorKind> {
        self.allocator_kind
    }

    crate fn has_global_allocator(&self) -> bool {
        self.has_global_allocator
    }

    pub fn report_unused_deps(&self, tcx: TyCtxt<'_>) {
        // We put the check for the option before the lint_level_at_node call
        // because the call mutates internal state and introducing it
        // leads to some ui tests failing.
        if !tcx.sess.opts.json_unused_externs {
            return;
        }
        let level = tcx
            .lint_level_at_node(lint::builtin::UNUSED_CRATE_DEPENDENCIES, rustc_hir::CRATE_HIR_ID)
            .0;
        if level != lint::Level::Allow {
            let unused_externs =
                self.unused_externs.iter().map(|ident| ident.to_ident_string()).collect::<Vec<_>>();
            let unused_externs = unused_externs.iter().map(String::as_str).collect::<Vec<&str>>();
            tcx.sess
                .parse_sess
                .span_diagnostic
                .emit_unused_externs(level.as_str(), &unused_externs);
        }
    }
}

impl<'a> CrateLoader<'a> {
    pub fn new(
        sess: &'a Session,
        metadata_loader: Box<MetadataLoaderDyn>,
        local_crate_name: &str,
    ) -> Self {
        let mut stable_crate_ids = FxHashMap::default();
        stable_crate_ids.insert(sess.local_stable_crate_id(), LOCAL_CRATE);

        CrateLoader {
            sess,
            metadata_loader,
            local_crate_name: Symbol::intern(local_crate_name),
            cstore: CStore {
                // We add an empty entry for LOCAL_CRATE (which maps to zero) in
                // order to make array indices in `metas` match with the
                // corresponding `CrateNum`. This first entry will always remain
                // `None`.
                metas: IndexVec::from_elem_n(None, 1),
                injected_panic_runtime: None,
                allocator_kind: None,
                has_global_allocator: false,
                stable_crate_ids,
                unused_externs: Vec::new(),
            },
            used_extern_options: Default::default(),
        }
    }

    pub fn cstore(&self) -> &CStore {
        &self.cstore
    }

    pub fn into_cstore(self) -> CStore {
        self.cstore
    }

    fn existing_match(&self, name: Symbol, hash: Option<Svh>, kind: PathKind) -> Option<CrateNum> {
        for (cnum, data) in self.cstore.iter_crate_data() {
            if data.name() != name {
                tracing::trace!("{} did not match {}", data.name(), name);
                continue;
            }

            match hash {
                Some(hash) if hash == data.hash() => return Some(cnum),
                Some(hash) => {
                    debug!("actual hash {} did not match expected {}", hash, data.hash());
                    continue;
                }
                None => {}
            }

            // When the hash is None we're dealing with a top-level dependency
            // in which case we may have a specification on the command line for
            // this library. Even though an upstream library may have loaded
            // something of the same name, we have to make sure it was loaded
            // from the exact same location as well.
            //
            // We're also sure to compare *paths*, not actual byte slices. The
            // `source` stores paths which are normalized which may be different
            // from the strings on the command line.
            let source = self.cstore.get_crate_data(cnum).cdata.source();
            if let Some(entry) = self.sess.opts.externs.get(name.as_str()) {
                // Only use `--extern crate_name=path` here, not `--extern crate_name`.
                if let Some(mut files) = entry.files() {
                    if files.any(|l| {
                        let l = l.canonicalized();
                        source.dylib.as_ref().map(|(p, _)| p) == Some(l)
                            || source.rlib.as_ref().map(|(p, _)| p) == Some(l)
                            || source.rmeta.as_ref().map(|(p, _)| p) == Some(l)
                    }) {
                        return Some(cnum);
                    }
                }
                continue;
            }

            // Alright, so we've gotten this far which means that `data` has the
            // right name, we don't have a hash, and we don't have a --extern
            // pointing for ourselves. We're still not quite yet done because we
            // have to make sure that this crate was found in the crate lookup
            // path (this is a top-level dependency) as we don't want to
            // implicitly load anything inside the dependency lookup path.
            let prev_kind = source
                .dylib
                .as_ref()
                .or(source.rlib.as_ref())
                .or(source.rmeta.as_ref())
                .expect("No sources for crate")
                .1;
            if kind.matches(prev_kind) {
                return Some(cnum);
            } else {
                debug!(
                    "failed to load existing crate {}; kind {:?} did not match prev_kind {:?}",
                    name, kind, prev_kind
                );
            }
        }

        None
    }

    fn verify_no_symbol_conflicts(&self, root: &CrateRoot<'_>) -> Result<(), CrateError> {
        // Check for (potential) conflicts with the local crate
        if self.sess.local_stable_crate_id() == root.stable_crate_id() {
            return Err(CrateError::SymbolConflictsCurrent(root.name()));
        }

        // Check for conflicts with any crate loaded so far
        for (_, other) in self.cstore.iter_crate_data() {
            // Same stable crate id but different SVH
            if other.stable_crate_id() == root.stable_crate_id() && other.hash() != root.hash() {
                return Err(CrateError::SymbolConflictsOthers(root.name()));
            }
        }

        Ok(())
    }

    fn verify_no_stable_crate_id_hash_conflicts(
        &mut self,
        root: &CrateRoot<'_>,
        cnum: CrateNum,
    ) -> Result<(), CrateError> {
        if let Some(existing) = self.cstore.stable_crate_ids.insert(root.stable_crate_id(), cnum) {
            let crate_name0 = root.name();
            let crate_name1 = self.cstore.get_crate_data(existing).name();
            return Err(CrateError::StableCrateIdCollision(crate_name0, crate_name1));
        }

        Ok(())
    }

    fn register_crate(
        &mut self,
        host_lib: Option<Library>,
        root: Option<&CratePaths>,
        lib: Library,
        dep_kind: CrateDepKind,
        name: Symbol,
    ) -> Result<CrateNum, CrateError> {
        let _prof_timer = self.sess.prof.generic_activity("metadata_register_crate");

        let Library { source, metadata } = lib;
        let crate_root = metadata.get_root();
        let host_hash = host_lib.as_ref().map(|lib| lib.metadata.get_root().hash());

        let private_dep =
            self.sess.opts.externs.get(name.as_str()).map_or(false, |e| e.is_private_dep);

        // Claim this crate number and cache it
        let cnum = self.cstore.alloc_new_crate_num();

        info!(
            "register crate `{}` (cnum = {}. private_dep = {})",
            crate_root.name(),
            cnum,
            private_dep
        );

        // Maintain a reference to the top most crate.
        // Stash paths for top-most crate locally if necessary.
        let crate_paths;
        let root = if let Some(root) = root {
            root
        } else {
            crate_paths = CratePaths::new(crate_root.name(), source.clone());
            &crate_paths
        };

        let cnum_map = self.resolve_crate_deps(root, &crate_root, &metadata, cnum, dep_kind)?;

        let raw_proc_macros = if crate_root.is_proc_macro_crate() {
            let temp_root;
            let (dlsym_source, dlsym_root) = match &host_lib {
                Some(host_lib) => (&host_lib.source, {
                    temp_root = host_lib.metadata.get_root();
                    &temp_root
                }),
                None => (&source, &crate_root),
            };
            let dlsym_dylib = dlsym_source.dylib.as_ref().expect("no dylib for a proc-macro crate");
            Some(self.dlsym_proc_macros(&dlsym_dylib.0, dlsym_root.stable_crate_id())?)
        } else {
            None
        };

        // Perform some verification *after* resolve_crate_deps() above is
        // known to have been successful. It seems that - in error cases - the
        // cstore can be in a temporarily invalid state between cnum allocation
        // and dependency resolution and the verification code would produce
        // ICEs in that case (see #83045).
        self.verify_no_symbol_conflicts(&crate_root)?;
        self.verify_no_stable_crate_id_hash_conflicts(&crate_root, cnum)?;

        let crate_metadata = CrateMetadata::new(
            self.sess,
            metadata,
            crate_root,
            raw_proc_macros,
            cnum,
            cnum_map,
            dep_kind,
            source,
            private_dep,
            host_hash,
        );

        self.cstore.set_crate_data(cnum, crate_metadata);

        Ok(cnum)
    }

    fn load_proc_macro<'b>(
        &self,
        locator: &mut CrateLocator<'b>,
        path_kind: PathKind,
        host_hash: Option<Svh>,
    ) -> Result<Option<(LoadResult, Option<Library>)>, CrateError>
    where
        'a: 'b,
    {
        // Use a new crate locator so trying to load a proc macro doesn't affect the error
        // message we emit
        let mut proc_macro_locator = locator.clone();

        // Try to load a proc macro
        proc_macro_locator.is_proc_macro = true;

        // Load the proc macro crate for the target
        let (locator, target_result) = if self.sess.opts.debugging_opts.dual_proc_macros {
            proc_macro_locator.reset();
            let result = match self.load(&mut proc_macro_locator)? {
                Some(LoadResult::Previous(cnum)) => {
                    return Ok(Some((LoadResult::Previous(cnum), None)));
                }
                Some(LoadResult::Loaded(library)) => Some(LoadResult::Loaded(library)),
                None => return Ok(None),
            };
            locator.hash = host_hash;
            // Use the locator when looking for the host proc macro crate, as that is required
            // so we want it to affect the error message
            (locator, result)
        } else {
            (&mut proc_macro_locator, None)
        };

        // Load the proc macro crate for the host

        locator.reset();
        locator.is_proc_macro = true;
        locator.target = &self.sess.host;
        locator.triple = TargetTriple::from_triple(config::host_triple());
        locator.filesearch = self.sess.host_filesearch(path_kind);

        let host_result = match self.load(locator)? {
            Some(host_result) => host_result,
            None => return Ok(None),
        };

        Ok(Some(if self.sess.opts.debugging_opts.dual_proc_macros {
            let host_result = match host_result {
                LoadResult::Previous(..) => {
                    panic!("host and target proc macros must be loaded in lock-step")
                }
                LoadResult::Loaded(library) => library,
            };
            (target_result.unwrap(), Some(host_result))
        } else {
            (host_result, None)
        }))
    }

    fn resolve_crate<'b>(
        &'b mut self,
        name: Symbol,
        span: Span,
        dep_kind: CrateDepKind,
    ) -> Option<CrateNum> {
        self.used_extern_options.insert(name);
        match self.maybe_resolve_crate(name, dep_kind, None) {
            Ok(cnum) => Some(cnum),
            Err(err) => {
                let missing_core =
                    self.maybe_resolve_crate(sym::core, CrateDepKind::Explicit, None).is_err();
                err.report(&self.sess, span, missing_core);
                None
            }
        }
    }

    fn maybe_resolve_crate<'b>(
        &'b mut self,
        name: Symbol,
        mut dep_kind: CrateDepKind,
        dep: Option<(&'b CratePaths, &'b CrateDep)>,
    ) -> Result<CrateNum, CrateError> {
        info!("resolving crate `{}`", name);
        if !name.as_str().is_ascii() {
            return Err(CrateError::NonAsciiName(name));
        }
        let (root, hash, host_hash, extra_filename, path_kind) = match dep {
            Some((root, dep)) => (
                Some(root),
                Some(dep.hash),
                dep.host_hash,
                Some(&dep.extra_filename[..]),
                PathKind::Dependency,
            ),
            None => (None, None, None, None, PathKind::Crate),
        };
        let result = if let Some(cnum) = self.existing_match(name, hash, path_kind) {
            (LoadResult::Previous(cnum), None)
        } else {
            info!("falling back to a load");
            let mut locator = CrateLocator::new(
                self.sess,
                &*self.metadata_loader,
                name,
                hash,
                extra_filename,
                false, // is_host
                path_kind,
            );

            match self.load(&mut locator)? {
                Some(res) => (res, None),
                None => {
                    dep_kind = CrateDepKind::MacrosOnly;
                    match self.load_proc_macro(&mut locator, path_kind, host_hash)? {
                        Some(res) => res,
                        None => return Err(locator.into_error(root.cloned())),
                    }
                }
            }
        };

        match result {
            (LoadResult::Previous(cnum), None) => {
                let data = self.cstore.get_crate_data(cnum);
                if data.is_proc_macro_crate() {
                    dep_kind = CrateDepKind::MacrosOnly;
                }
                data.update_dep_kind(|data_dep_kind| cmp::max(data_dep_kind, dep_kind));
                Ok(cnum)
            }
            (LoadResult::Loaded(library), host_library) => {
                self.register_crate(host_library, root, library, dep_kind, name)
            }
            _ => panic!(),
        }
    }

    fn load(&self, locator: &mut CrateLocator<'_>) -> Result<Option<LoadResult>, CrateError> {
        let library = match locator.maybe_load_library_crate()? {
            Some(library) => library,
            None => return Ok(None),
        };

        // In the case that we're loading a crate, but not matching
        // against a hash, we could load a crate which has the same hash
        // as an already loaded crate. If this is the case prevent
        // duplicates by just using the first crate.
        //
        // Note that we only do this for target triple crates, though, as we
        // don't want to match a host crate against an equivalent target one
        // already loaded.
        let root = library.metadata.get_root();
        // FIXME: why is this condition necessary? It was adding in #33625 but I
        // don't know why and the original author doesn't remember ...
        let can_reuse_cratenum =
            locator.triple == self.sess.opts.target_triple || locator.is_proc_macro;
        Ok(Some(if can_reuse_cratenum {
            let mut result = LoadResult::Loaded(library);
            for (cnum, data) in self.cstore.iter_crate_data() {
                if data.name() == root.name() && root.hash() == data.hash() {
                    assert!(locator.hash.is_none());
                    info!("load success, going to previous cnum: {}", cnum);
                    result = LoadResult::Previous(cnum);
                    break;
                }
            }
            result
        } else {
            LoadResult::Loaded(library)
        }))
    }

    fn update_extern_crate(&self, cnum: CrateNum, extern_crate: ExternCrate) {
        let cmeta = self.cstore.get_crate_data(cnum);
        if cmeta.update_extern_crate(extern_crate) {
            // Propagate the extern crate info to dependencies if it was updated.
            let extern_crate = ExternCrate { dependency_of: cnum, ..extern_crate };
            for &dep_cnum in cmeta.dependencies().iter() {
                self.update_extern_crate(dep_cnum, extern_crate);
            }
        }
    }

    // Go through the crate metadata and load any crates that it references
    fn resolve_crate_deps(
        &mut self,
        root: &CratePaths,
        crate_root: &CrateRoot<'_>,
        metadata: &MetadataBlob,
        krate: CrateNum,
        dep_kind: CrateDepKind,
    ) -> Result<CrateNumMap, CrateError> {
        debug!("resolving deps of external crate");
        if crate_root.is_proc_macro_crate() {
            return Ok(CrateNumMap::new());
        }

        // The map from crate numbers in the crate we're resolving to local crate numbers.
        // We map 0 and all other holes in the map to our parent crate. The "additional"
        // self-dependencies should be harmless.
        let deps = crate_root.decode_crate_deps(metadata);
        let mut crate_num_map = CrateNumMap::with_capacity(1 + deps.len());
        crate_num_map.push(krate);
        for dep in deps {
            info!(
                "resolving dep crate {} hash: `{}` extra filename: `{}`",
                dep.name, dep.hash, dep.extra_filename
            );
            let dep_kind = match dep_kind {
                CrateDepKind::MacrosOnly => CrateDepKind::MacrosOnly,
                _ => dep.kind,
            };
            let cnum = self.maybe_resolve_crate(dep.name, dep_kind, Some((root, &dep)))?;
            crate_num_map.push(cnum);
        }

        debug!("resolve_crate_deps: cnum_map for {:?} is {:?}", krate, crate_num_map);
        Ok(crate_num_map)
    }

    fn dlsym_proc_macros(
        &self,
        path: &Path,
        stable_crate_id: StableCrateId,
    ) -> Result<&'static [ProcMacro], CrateError> {
        // Make sure the path contains a / or the linker will search for it.
        let path = env::current_dir().unwrap().join(path);
        let lib = unsafe { libloading::Library::new(path) }
            .map_err(|err| CrateError::DlOpen(err.to_string()))?;

        let sym_name = self.sess.generate_proc_macro_decls_symbol(stable_crate_id);
        let sym = unsafe { lib.get::<*const &[ProcMacro]>(sym_name.as_bytes()) }
            .map_err(|err| CrateError::DlSym(err.to_string()))?;

        // Intentionally leak the dynamic library. We can't ever unload it
        // since the library can make things that will live arbitrarily long.
        let sym = unsafe { sym.into_raw() };
        std::mem::forget(lib);

        Ok(unsafe { **sym })
    }

    fn inject_panic_runtime(&mut self, krate: &ast::Crate) {
        // If we're only compiling an rlib, then there's no need to select a
        // panic runtime, so we just skip this section entirely.
        let any_non_rlib = self.sess.crate_types().iter().any(|ct| *ct != CrateType::Rlib);
        if !any_non_rlib {
            info!("panic runtime injection skipped, only generating rlib");
            return;
        }

        // If we need a panic runtime, we try to find an existing one here. At
        // the same time we perform some general validation of the DAG we've got
        // going such as ensuring everything has a compatible panic strategy.
        //
        // The logic for finding the panic runtime here is pretty much the same
        // as the allocator case with the only addition that the panic strategy
        // compilation mode also comes into play.
        let desired_strategy = self.sess.panic_strategy();
        let mut runtime_found = false;
        let mut needs_panic_runtime =
            self.sess.contains_name(&krate.attrs, sym::needs_panic_runtime);

        for (cnum, data) in self.cstore.iter_crate_data() {
            needs_panic_runtime = needs_panic_runtime || data.needs_panic_runtime();
            if data.is_panic_runtime() {
                // Inject a dependency from all #![needs_panic_runtime] to this
                // #![panic_runtime] crate.
                self.inject_dependency_if(cnum, "a panic runtime", &|data| {
                    data.needs_panic_runtime()
                });
                runtime_found = runtime_found || data.dep_kind() == CrateDepKind::Explicit;
            }
        }

        // If an explicitly linked and matching panic runtime was found, or if
        // we just don't need one at all, then we're done here and there's
        // nothing else to do.
        if !needs_panic_runtime || runtime_found {
            return;
        }

        // By this point we know that we (a) need a panic runtime and (b) no
        // panic runtime was explicitly linked. Here we just load an appropriate
        // default runtime for our panic strategy and then inject the
        // dependencies.
        //
        // We may resolve to an already loaded crate (as the crate may not have
        // been explicitly linked prior to this) and we may re-inject
        // dependencies again, but both of those situations are fine.
        //
        // Also note that we have yet to perform validation of the crate graph
        // in terms of everyone has a compatible panic runtime format, that's
        // performed later as part of the `dependency_format` module.
        let name = match desired_strategy {
            PanicStrategy::Unwind => sym::panic_unwind,
            PanicStrategy::Abort => sym::panic_abort,
        };
        info!("panic runtime not found -- loading {}", name);

        let Some(cnum) = self.resolve_crate(name, DUMMY_SP, CrateDepKind::Implicit) else { return; };
        let data = self.cstore.get_crate_data(cnum);

        // Sanity check the loaded crate to ensure it is indeed a panic runtime
        // and the panic strategy is indeed what we thought it was.
        if !data.is_panic_runtime() {
            self.sess.err(&format!("the crate `{}` is not a panic runtime", name));
        }
        if data.panic_strategy() != desired_strategy {
            self.sess.err(&format!(
                "the crate `{}` does not have the panic \
                                    strategy `{}`",
                name,
                desired_strategy.desc()
            ));
        }

        self.cstore.injected_panic_runtime = Some(cnum);
        self.inject_dependency_if(cnum, "a panic runtime", &|data| data.needs_panic_runtime());
    }

    fn inject_profiler_runtime(&mut self, krate: &ast::Crate) {
        if self.sess.opts.debugging_opts.no_profiler_runtime
            || !(self.sess.instrument_coverage()
                || self.sess.opts.debugging_opts.profile
                || self.sess.opts.cg.profile_generate.enabled())
        {
            return;
        }

        info!("loading profiler");

        let name = Symbol::intern(&self.sess.opts.debugging_opts.profiler_runtime);
        if name == sym::profiler_builtins && self.sess.contains_name(&krate.attrs, sym::no_core) {
            self.sess.err(
                "`profiler_builtins` crate (required by compiler options) \
                        is not compatible with crate attribute `#![no_core]`",
            );
        }

        let Some(cnum) = self.resolve_crate(name, DUMMY_SP, CrateDepKind::Implicit) else { return; };
        let data = self.cstore.get_crate_data(cnum);

        // Sanity check the loaded crate to ensure it is indeed a profiler runtime
        if !data.is_profiler_runtime() {
            self.sess.err(&format!("the crate `{}` is not a profiler runtime", name));
        }
    }

    fn inject_allocator_crate(&mut self, krate: &ast::Crate) {
        self.cstore.has_global_allocator = match &*global_allocator_spans(&self.sess, krate) {
            [span1, span2, ..] => {
                self.sess
                    .struct_span_err(*span2, "cannot define multiple global allocators")
                    .span_label(*span2, "cannot define a new global allocator")
                    .span_label(*span1, "previous global allocator defined here")
                    .emit();
                true
            }
            spans => !spans.is_empty(),
        };

        // Check to see if we actually need an allocator. This desire comes
        // about through the `#![needs_allocator]` attribute and is typically
        // written down in liballoc.
        if !self.sess.contains_name(&krate.attrs, sym::needs_allocator)
            && !self.cstore.iter_crate_data().any(|(_, data)| data.needs_allocator())
        {
            return;
        }

        // At this point we've determined that we need an allocator. Let's see
        // if our compilation session actually needs an allocator based on what
        // we're emitting.
        let all_rlib = self.sess.crate_types().iter().all(|ct| matches!(*ct, CrateType::Rlib));
        if all_rlib {
            return;
        }

        // Ok, we need an allocator. Not only that but we're actually going to
        // create an artifact that needs one linked in. Let's go find the one
        // that we're going to link in.
        //
        // First up we check for global allocators. Look at the crate graph here
        // and see what's a global allocator, including if we ourselves are a
        // global allocator.
        let mut global_allocator =
            self.cstore.has_global_allocator.then(|| Symbol::intern("this crate"));
        for (_, data) in self.cstore.iter_crate_data() {
            if data.has_global_allocator() {
                match global_allocator {
                    Some(other_crate) => self.sess.err(&format!(
                        "the `#[global_allocator]` in {} conflicts with global allocator in: {}",
                        other_crate,
                        data.name()
                    )),
                    None => global_allocator = Some(data.name()),
                }
            }
        }

        if global_allocator.is_some() {
            self.cstore.allocator_kind = Some(AllocatorKind::Global);
            return;
        }

        // Ok we haven't found a global allocator but we still need an
        // allocator. At this point our allocator request is typically fulfilled
        // by the standard library, denoted by the `#![default_lib_allocator]`
        // attribute.
        if !self.sess.contains_name(&krate.attrs, sym::default_lib_allocator)
            && !self.cstore.iter_crate_data().any(|(_, data)| data.has_default_lib_allocator())
        {
            self.sess.err(
                "no global memory allocator found but one is required; link to std or add \
                 `#[global_allocator]` to a static item that implements the GlobalAlloc trait",
            );
        }
        self.cstore.allocator_kind = Some(AllocatorKind::Default);
    }

    fn inject_dependency_if(
        &self,
        krate: CrateNum,
        what: &str,
        needs_dep: &dyn Fn(&CrateMetadata) -> bool,
    ) {
        // don't perform this validation if the session has errors, as one of
        // those errors may indicate a circular dependency which could cause
        // this to stack overflow.
        if self.sess.has_errors() {
            return;
        }

        // Before we inject any dependencies, make sure we don't inject a
        // circular dependency by validating that this crate doesn't
        // transitively depend on any crates satisfying `needs_dep`.
        for dep in self.cstore.crate_dependencies_in_reverse_postorder(krate) {
            let data = self.cstore.get_crate_data(dep);
            if needs_dep(&data) {
                self.sess.err(&format!(
                    "the crate `{}` cannot depend \
                                        on a crate that needs {}, but \
                                        it depends on `{}`",
                    self.cstore.get_crate_data(krate).name(),
                    what,
                    data.name()
                ));
            }
        }

        // All crates satisfying `needs_dep` do not explicitly depend on the
        // crate provided for this compile, but in order for this compilation to
        // be successfully linked we need to inject a dependency (to order the
        // crates on the command line correctly).
        for (cnum, data) in self.cstore.iter_crate_data() {
            if needs_dep(data) {
                info!("injecting a dep from {} to {}", cnum, krate);
                data.add_dependency(krate);
            }
        }
    }

    fn report_unused_deps(&mut self, krate: &ast::Crate) {
        // Make a point span rather than covering the whole file
        let span = krate.span.shrink_to_lo();
        // Complain about anything left over
        for (name, entry) in self.sess.opts.externs.iter() {
            if let ExternLocation::FoundInLibrarySearchDirectories = entry.location {
                // Don't worry about pathless `--extern foo` sysroot references
                continue;
            }
            let name_interned = Symbol::intern(name);
            if self.used_extern_options.contains(&name_interned) {
                continue;
            }

            // Got a real unused --extern
            if self.sess.opts.json_unused_externs {
                self.cstore.unused_externs.push(name_interned);
                continue;
            }

            let diag = match self.sess.opts.extern_dep_specs.get(name) {
                Some(loc) => BuiltinLintDiagnostics::ExternDepSpec(name.clone(), loc.into()),
                None => {
                    // If we don't have a specific location, provide a json encoding of the `--extern`
                    // option.
                    let meta: BTreeMap<String, String> =
                        std::iter::once(("name".to_string(), name.to_string())).collect();
                    BuiltinLintDiagnostics::ExternDepSpec(
                        name.clone(),
                        ExternDepSpec::Json(meta.to_json()),
                    )
                }
            };
            self.sess.parse_sess.buffer_lint_with_diagnostic(
                    lint::builtin::UNUSED_CRATE_DEPENDENCIES,
                    span,
                    ast::CRATE_NODE_ID,
                    &format!(
                        "external crate `{}` unused in `{}`: remove the dependency or add `use {} as _;`",
                        name,
                        self.local_crate_name,
                        name),
                    diag,
                );
        }
    }

    pub fn postprocess(&mut self, krate: &ast::Crate) {
        self.inject_profiler_runtime(krate);
        self.inject_allocator_crate(krate);
        self.inject_panic_runtime(krate);

        self.report_unused_deps(krate);

        info!("{:?}", CrateDump(&self.cstore));
    }

    pub fn process_extern_crate(
        &mut self,
        item: &ast::Item,
        definitions: &Definitions,
        def_id: LocalDefId,
    ) -> Option<CrateNum> {
        match item.kind {
            ast::ItemKind::ExternCrate(orig_name) => {
                debug!(
                    "resolving extern crate stmt. ident: {} orig_name: {:?}",
                    item.ident, orig_name
                );
                let name = match orig_name {
                    Some(orig_name) => {
                        validate_crate_name(self.sess, orig_name.as_str(), Some(item.span));
                        orig_name
                    }
                    None => item.ident.name,
                };
                let dep_kind = if self.sess.contains_name(&item.attrs, sym::no_link) {
                    CrateDepKind::MacrosOnly
                } else {
                    CrateDepKind::Explicit
                };

                let cnum = self.resolve_crate(name, item.span, dep_kind)?;

                let path_len = definitions.def_path(def_id).data.len();
                self.update_extern_crate(
                    cnum,
                    ExternCrate {
                        src: ExternCrateSource::Extern(def_id.to_def_id()),
                        span: item.span,
                        path_len,
                        dependency_of: LOCAL_CRATE,
                    },
                );
                Some(cnum)
            }
            _ => bug!(),
        }
    }

    pub fn process_path_extern(&mut self, name: Symbol, span: Span) -> Option<CrateNum> {
        let cnum = self.resolve_crate(name, span, CrateDepKind::Explicit)?;

        self.update_extern_crate(
            cnum,
            ExternCrate {
                src: ExternCrateSource::Path,
                span,
                // to have the least priority in `update_extern_crate`
                path_len: usize::MAX,
                dependency_of: LOCAL_CRATE,
            },
        );

        Some(cnum)
    }

    pub fn maybe_process_path_extern(&mut self, name: Symbol) -> Option<CrateNum> {
        self.maybe_resolve_crate(name, CrateDepKind::Explicit, None).ok()
    }
}

fn global_allocator_spans(sess: &Session, krate: &ast::Crate) -> Vec<Span> {
    struct Finder<'a> {
        sess: &'a Session,
        name: Symbol,
        spans: Vec<Span>,
    }
    impl<'ast, 'a> visit::Visitor<'ast> for Finder<'a> {
        fn visit_item(&mut self, item: &'ast ast::Item) {
            if item.ident.name == self.name
                && self.sess.contains_name(&item.attrs, sym::rustc_std_internal_symbol)
            {
                self.spans.push(item.span);
            }
            visit::walk_item(self, item)
        }
    }

    let name = Symbol::intern(&AllocatorKind::Global.fn_name(sym::alloc));
    let mut f = Finder { sess, name, spans: Vec::new() };
    visit::walk_crate(&mut f, krate);
    f.spans
}
