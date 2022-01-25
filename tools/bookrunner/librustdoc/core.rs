use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_data_structures::sync::{self, Lrc};
use rustc_errors::emitter::{Emitter, EmitterWriter};
use rustc_errors::json::JsonEmitter;
use rustc_feature::UnstableFeatures;
use rustc_hir::def::Res;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{HirId, Path};
use rustc_interface::interface;
use rustc_middle::hir::nested_filter;
use rustc_middle::middle::privacy::AccessLevels;
use rustc_middle::ty::{ParamEnv, Ty, TyCtxt};
use rustc_resolve as resolve;
use rustc_session::config::{self, CrateType, ErrorOutputType};
use rustc_session::lint;
use rustc_session::DiagnosticOutput;
use rustc_session::Session;
use rustc_span::symbol::sym;
use rustc_span::{source_map, Span};

use std::cell::RefCell;
use std::lazy::SyncLazy;
use std::mem;
use std::rc::Rc;

use crate::clean::inline::build_external_trait;
use crate::clean::{self, ItemId, TraitWithExtraInfo};
use crate::config::{Options as RustdocOptions, OutputFormat, RenderOptions};
use crate::formats::cache::Cache;
use crate::passes::{self, Condition::*};

crate use rustc_session::config::{DebuggingOptions, Input, Options};

crate struct ResolverCaches {
    pub all_traits: Option<Vec<DefId>>,
    pub all_trait_impls: Option<Vec<DefId>>,
}

crate struct DocContext<'tcx> {
    crate tcx: TyCtxt<'tcx>,
    /// Name resolver. Used for intra-doc links.
    ///
    /// The `Rc<RefCell<...>>` wrapping is needed because that is what's returned by
    /// [`rustc_interface::Queries::expansion()`].
    // FIXME: see if we can get rid of this RefCell somehow
    crate resolver: Rc<RefCell<interface::BoxedResolver>>,
    crate resolver_caches: ResolverCaches,
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
    /// Auto-trait or blanket impls processed so far, as `(self_ty, trait_def_id)`.
    // FIXME(eddyb) make this a `ty::TraitRef<'tcx>` set.
    crate generated_synthetics: FxHashSet<(Ty<'tcx>, DefId)>,
    crate auto_traits: Vec<DefId>,
    /// The options given to rustdoc that could be relevant to a pass.
    crate render_options: RenderOptions,
    /// The traits in scope for a given module.
    ///
    /// See `collect_intra_doc_links::traits_implemented_by` for more details.
    /// `map<module, set<trait>>`
    crate module_trait_cache: FxHashMap<DefId, FxHashSet<DefId>>,
    /// This same cache is used throughout rustdoc, including in [`crate::html::render`].
    crate cache: Cache,
    /// Used by [`clean::inline`] to tell if an item has already been inlined.
    crate inlined: FxHashSet<ItemId>,
    /// Used by `calculate_doc_coverage`.
    crate output_format: OutputFormat,
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

    crate fn enter_resolver<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut resolve::Resolver<'_>) -> R,
    {
        self.resolver.borrow_mut().access(f)
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

    crate fn with_all_traits(&mut self, f: impl FnOnce(&mut Self, &[DefId])) {
        let all_traits = self.resolver_caches.all_traits.take();
        f(self, all_traits.as_ref().expect("`all_traits` are already borrowed"));
        self.resolver_caches.all_traits = all_traits;
    }

    crate fn with_all_trait_impls(&mut self, f: impl FnOnce(&mut Self, &[DefId])) {
        let all_trait_impls = self.resolver_caches.all_trait_impls.take();
        f(self, all_trait_impls.as_ref().expect("`all_trait_impls` are already borrowed"));
        self.resolver_caches.all_trait_impls = all_trait_impls;
    }
}

/// Creates a new diagnostic `Handler` that can be used to emit warnings and errors.
///
/// If the given `error_format` is `ErrorOutputType::Json` and no `SourceMap` is given, a new one
/// will be created for the handler.
crate fn new_handler(
    error_format: ErrorOutputType,
    source_map: Option<Lrc<source_map::SourceMap>>,
    debugging_opts: &DebuggingOptions,
) -> rustc_errors::Handler {
    let emitter: Box<dyn Emitter + sync::Send> = match error_format {
        ErrorOutputType::HumanReadable(kind) => {
            let (short, color_config) = kind.unzip();
            Box::new(
                EmitterWriter::stderr(
                    color_config,
                    source_map.map(|sm| sm as _),
                    short,
                    debugging_opts.teach,
                    debugging_opts.terminal_width,
                    false,
                )
                .ui_testing(debugging_opts.ui_testing),
            )
        }
        ErrorOutputType::Json { pretty, json_rendered } => {
            let source_map = source_map.unwrap_or_else(|| {
                Lrc::new(source_map::SourceMap::new(source_map::FilePathMapping::empty()))
            });
            Box::new(
                JsonEmitter::stderr(
                    None,
                    source_map,
                    pretty,
                    json_rendered,
                    debugging_opts.terminal_width,
                    false,
                )
                .ui_testing(debugging_opts.ui_testing),
            )
        }
    };

    rustc_errors::Handler::with_emitter_and_flags(
        emitter,
        debugging_opts.diagnostic_handler_flags(true),
    )
}

/// Parse, resolve, and typecheck the given crate.
crate fn create_config(
    RustdocOptions {
        input,
        crate_name,
        proc_macro_crate,
        error_format,
        libs,
        externs,
        mut cfgs,
        codegen_options,
        debugging_opts,
        target,
        edition,
        maybe_sysroot,
        lint_opts,
        describe_lints,
        lint_cap,
        ..
    }: RustdocOptions,
) -> rustc_interface::Config {
    // Add the doc cfg into the doc build.
    cfgs.push("doc".to_string());

    let cpath = Some(input.clone());
    let input = Input::File(input);

    // By default, rustdoc ignores all lints.
    // Specifically unblock lints relevant to documentation or the lint machinery itself.
    let mut lints_to_show = vec![
        // it's unclear whether these should be part of rustdoc directly (#77364)
        rustc_lint::builtin::MISSING_DOCS.name.to_string(),
        rustc_lint::builtin::INVALID_DOC_ATTRIBUTES.name.to_string(),
        // these are definitely not part of rustdoc, but we want to warn on them anyway.
        rustc_lint::builtin::RENAMED_AND_REMOVED_LINTS.name.to_string(),
        rustc_lint::builtin::UNKNOWN_LINTS.name.to_string(),
    ];
    lints_to_show.extend(crate::lint::RUSTDOC_LINTS.iter().map(|lint| lint.name.to_string()));

    let (lint_opts, lint_caps) = crate::lint::init_lints(lints_to_show, lint_opts, |lint| {
        Some((lint.name_lower(), lint::Allow))
    });

    let crate_types =
        if proc_macro_crate { vec![CrateType::ProcMacro] } else { vec![CrateType::Rlib] };
    // plays with error output here!
    let sessopts = config::Options {
        maybe_sysroot,
        search_paths: libs,
        crate_types,
        lint_opts,
        lint_cap,
        cg: codegen_options,
        externs,
        target_triple: target,
        unstable_features: UnstableFeatures::from_environment(crate_name.as_deref()),
        actually_rustdoc: true,
        debugging_opts,
        error_format,
        edition,
        describe_lints,
        crate_name,
        ..Options::default()
    };

    interface::Config {
        opts: sessopts,
        crate_cfg: interface::parse_cfgspecs(cfgs),
        input,
        input_path: cpath,
        output_file: None,
        output_dir: None,
        file_loader: None,
        diagnostic_output: DiagnosticOutput::Default,
        stderr: None,
        lint_caps,
        parse_sess_created: None,
        register_lints: Some(box crate::lint::register_lints),
        override_queries: Some(|_sess, providers, _external_providers| {
            // Most lints will require typechecking, so just don't run them.
            providers.lint_mod = |_, _| {};
            // Prevent `rustc_typeck::check_crate` from calling `typeck` on all bodies.
            providers.typeck_item_bodies = |_, _| {};
            // hack so that `used_trait_imports` won't try to call typeck
            providers.used_trait_imports = |_, _| {
                static EMPTY_SET: SyncLazy<FxHashSet<LocalDefId>> =
                    SyncLazy::new(FxHashSet::default);
                &EMPTY_SET
            };
            // In case typeck does end up being called, don't ICE in case there were name resolution errors
            providers.typeck = move |tcx, def_id| {
                // Closures' tables come from their outermost function,
                // as they are part of the same "inference environment".
                // This avoids emitting errors for the parent twice (see similar code in `typeck_with_fallback`)
                let typeck_root_def_id = tcx.typeck_root_def_id(def_id.to_def_id()).expect_local();
                if typeck_root_def_id != def_id {
                    return tcx.typeck(typeck_root_def_id);
                }

                let hir = tcx.hir();
                let body = hir.body(hir.body_owned_by(hir.local_def_id_to_hir_id(def_id)));
                debug!("visiting body for {:?}", def_id);
                EmitIgnoredResolutionErrors::new(tcx).visit_body(body);
                (rustc_interface::DEFAULT_QUERY_PROVIDERS.typeck)(tcx, def_id)
            };
        }),
        make_codegen_backend: None,
        registry: rustc_driver::diagnostics_registry(),
    }
}

crate fn run_global_ctxt(
    tcx: TyCtxt<'_>,
    resolver: Rc<RefCell<interface::BoxedResolver>>,
    resolver_caches: ResolverCaches,
    show_coverage: bool,
    render_options: RenderOptions,
    output_format: OutputFormat,
) -> (clean::Crate, RenderOptions, Cache) {
    // Certain queries assume that some checks were run elsewhere
    // (see https://github.com/rust-lang/rust/pull/73566#issuecomment-656954425),
    // so type-check everything other than function bodies in this crate before running lints.

    // NOTE: this does not call `tcx.analysis()` so that we won't
    // typeck function bodies or run the default rustc lints.
    // (see `override_queries` in the `config`)

    // HACK(jynelson) this calls an _extremely_ limited subset of `typeck`
    // and might break if queries change their assumptions in the future.

    // NOTE: This is copy/pasted from typeck/lib.rs and should be kept in sync with those changes.
    tcx.sess.time("item_types_checking", || {
        tcx.hir().for_each_module(|module| tcx.ensure().check_mod_item_types(module))
    });
    tcx.sess.abort_if_errors();
    tcx.sess.time("missing_docs", || {
        rustc_lint::check_crate(tcx, rustc_lint::builtin::MissingDoc::new);
    });
    tcx.sess.time("check_mod_attrs", || {
        tcx.hir().for_each_module(|module| tcx.ensure().check_mod_attrs(module))
    });
    rustc_passes::stability::check_unused_or_stable_features(tcx);

    let auto_traits = resolver_caches
        .all_traits
        .as_ref()
        .expect("`all_traits` are already borrowed")
        .iter()
        .copied()
        .filter(|&trait_def_id| tcx.trait_is_auto(trait_def_id))
        .collect();
    let access_levels = AccessLevels {
        map: tcx.privacy_access_levels(()).map.iter().map(|(k, v)| (k.to_def_id(), *v)).collect(),
    };

    let mut ctxt = DocContext {
        tcx,
        resolver,
        resolver_caches,
        param_env: ParamEnv::empty(),
        external_traits: Default::default(),
        active_extern_traits: Default::default(),
        substs: Default::default(),
        impl_trait_bounds: Default::default(),
        generated_synthetics: Default::default(),
        auto_traits,
        module_trait_cache: FxHashMap::default(),
        cache: Cache::new(access_levels, render_options.document_private),
        inlined: FxHashSet::default(),
        output_format,
        render_options,
    };

    // Small hack to force the Sized trait to be present.
    //
    // Note that in case of `#![no_core]`, the trait is not available.
    if let Some(sized_trait_did) = ctxt.tcx.lang_items().sized_trait() {
        let mut sized_trait = build_external_trait(&mut ctxt, sized_trait_did);
        sized_trait.is_auto = true;
        ctxt.external_traits
            .borrow_mut()
            .insert(sized_trait_did, TraitWithExtraInfo { trait_: sized_trait, is_notable: false });
    }

    debug!("crate: {:?}", tcx.hir().krate());

    let mut krate = tcx.sess.time("clean_crate", || clean::krate(&mut ctxt));

    if krate.module.doc_value().map(|d| d.is_empty()).unwrap_or(true) {
        let help = format!(
            "The following guide may be of use:\n\
            {}/rustdoc/how-to-write-documentation.html",
            crate::DOC_RUST_LANG_ORG_CHANNEL
        );
        tcx.struct_lint_node(
            crate::lint::MISSING_CRATE_LEVEL_DOCS,
            DocContext::as_local_hir_id(tcx, krate.module.def_id).unwrap(),
            |lint| {
                let mut diag =
                    lint.build("no documentation found for this crate's top-level module");
                diag.help(&help);
                diag.emit();
            },
        );
    }

    fn report_deprecated_attr(name: &str, diag: &rustc_errors::Handler, sp: Span) {
        let mut msg =
            diag.struct_span_warn(sp, &format!("the `#![doc({})]` attribute is deprecated", name));
        msg.note(
            "see issue #44136 <https://github.com/rust-lang/rust/issues/44136> \
            for more information",
        );

        if name == "no_default_passes" {
            msg.help("`#![doc(no_default_passes)]` no longer functions; you may want to use `#![doc(document_private_items)]`");
        } else if name.starts_with("passes") {
            msg.help("`#![doc(passes = \"...\")]` no longer functions; you may want to use `#![doc(document_private_items)]`");
        } else if name.starts_with("plugins") {
            msg.warn("`#![doc(plugins = \"...\")]` no longer functions; see CVE-2018-1000622 <https://nvd.nist.gov/vuln/detail/CVE-2018-1000622>");
        }

        msg.emit();
    }

    // Process all of the crate attributes, extracting plugin metadata along
    // with the passes which we are supposed to run.
    for attr in krate.module.attrs.lists(sym::doc) {
        let diag = ctxt.sess().diagnostic();

        let name = attr.name_or_empty();
        // `plugins = "..."`, `no_default_passes`, and `passes = "..."` have no effect
        if attr.is_word() && name == sym::no_default_passes {
            report_deprecated_attr("no_default_passes", diag, attr.span());
        } else if attr.value_str().is_some() {
            match name {
                sym::passes => {
                    report_deprecated_attr("passes = \"...\"", diag, attr.span());
                }
                sym::plugins => {
                    report_deprecated_attr("plugins = \"...\"", diag, attr.span());
                }
                _ => (),
            }
        }

        if attr.is_word() && name == sym::document_private_items {
            ctxt.render_options.document_private = true;
        }
    }

    info!("Executing passes");

    for p in passes::defaults(show_coverage) {
        let run = match p.condition {
            Always => true,
            WhenDocumentPrivate => ctxt.render_options.document_private,
            WhenNotDocumentPrivate => !ctxt.render_options.document_private,
            WhenNotDocumentHidden => !ctxt.render_options.document_hidden,
        };
        if run {
            debug!("running pass {}", p.pass.name);
            krate = tcx.sess.time(p.pass.name, || (p.pass.run)(krate, &mut ctxt));
        }
    }

    if tcx.sess.diagnostic().has_errors_or_lint_errors() {
        rustc_errors::FatalError.raise();
    }

    krate = tcx.sess.time("create_format_cache", || Cache::populate(&mut ctxt, krate));

    (krate, ctxt.render_options, ctxt.cache)
}

/// Due to <https://github.com/rust-lang/rust/pull/73566>,
/// the name resolution pass may find errors that are never emitted.
/// If typeck is called after this happens, then we'll get an ICE:
/// 'Res::Error found but not reported'. To avoid this, emit the errors now.
struct EmitIgnoredResolutionErrors<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> EmitIgnoredResolutionErrors<'tcx> {
    fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self { tcx }
    }
}

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
