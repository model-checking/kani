//! A bunch of methods and structures more or less related to resolving macros and
//! interface provided by `Resolver` to macro expander.

use crate::imports::ImportResolver;
use crate::Namespace::*;
use crate::{AmbiguityError, AmbiguityErrorMisc, AmbiguityKind, BuiltinMacroState, Determinacy};
use crate::{CrateLint, DeriveData, ParentScope, ResolutionError, Resolver, Scope, ScopeSet, Weak};
use crate::{ModuleKind, ModuleOrUniformRoot, NameBinding, PathResult, Segment, ToNameBinding};
use rustc_ast::{self as ast, Inline, ItemKind, ModKind, NodeId};
use rustc_ast_lowering::ResolverAstLowering;
use rustc_ast_pretty::pprust;
use rustc_attr::StabilityLevel;
use rustc_data_structures::fx::FxHashSet;
use rustc_data_structures::ptr_key::PtrKey;
use rustc_data_structures::sync::Lrc;
use rustc_errors::struct_span_err;
use rustc_expand::base::{Annotatable, DeriveResolutions, Indeterminate, ResolverExpand};
use rustc_expand::base::{SyntaxExtension, SyntaxExtensionKind};
use rustc_expand::compile_declarative_macro;
use rustc_expand::expand::{AstFragment, Invocation, InvocationKind, SupportsMacroExpansion};
use rustc_feature::is_builtin_attr_name;
use rustc_hir::def::{self, DefKind, NonMacroAttrKind};
use rustc_hir::def_id::{CrateNum, LocalDefId};
use rustc_hir::PrimTy;
use rustc_middle::middle::stability;
use rustc_middle::ty;
use rustc_session::lint::builtin::{LEGACY_DERIVE_HELPERS, PROC_MACRO_DERIVE_RESOLUTION_FALLBACK};
use rustc_session::lint::builtin::{SOFT_UNSTABLE, UNUSED_MACROS};
use rustc_session::lint::BuiltinLintDiagnostics;
use rustc_session::parse::feature_err;
use rustc_session::Session;
use rustc_span::edition::Edition;
use rustc_span::hygiene::{self, ExpnData, ExpnKind, LocalExpnId};
use rustc_span::hygiene::{AstPass, MacroKind};
use rustc_span::symbol::{kw, sym, Ident, Symbol};
use rustc_span::{Span, DUMMY_SP};
use std::cell::Cell;
use std::{mem, ptr};

type Res = def::Res<NodeId>;

/// Binding produced by a `macro_rules` item.
/// Not modularized, can shadow previous `macro_rules` bindings, etc.
#[derive(Debug)]
pub struct MacroRulesBinding<'a> {
    crate binding: &'a NameBinding<'a>,
    /// `macro_rules` scope into which the `macro_rules` item was planted.
    crate parent_macro_rules_scope: MacroRulesScopeRef<'a>,
    crate ident: Ident,
}

/// The scope introduced by a `macro_rules!` macro.
/// This starts at the macro's definition and ends at the end of the macro's parent
/// module (named or unnamed), or even further if it escapes with `#[macro_use]`.
/// Some macro invocations need to introduce `macro_rules` scopes too because they
/// can potentially expand into macro definitions.
#[derive(Copy, Clone, Debug)]
pub enum MacroRulesScope<'a> {
    /// Empty "root" scope at the crate start containing no names.
    Empty,
    /// The scope introduced by a `macro_rules!` macro definition.
    Binding(&'a MacroRulesBinding<'a>),
    /// The scope introduced by a macro invocation that can potentially
    /// create a `macro_rules!` macro definition.
    Invocation(LocalExpnId),
}

/// `macro_rules!` scopes are always kept by reference and inside a cell.
/// The reason is that we update scopes with value `MacroRulesScope::Invocation(invoc_id)`
/// in-place after `invoc_id` gets expanded.
/// This helps to avoid uncontrollable growth of `macro_rules!` scope chains,
/// which usually grow lineraly with the number of macro invocations
/// in a module (including derives) and hurt performance.
pub(crate) type MacroRulesScopeRef<'a> = PtrKey<'a, Cell<MacroRulesScope<'a>>>;

// Macro namespace is separated into two sub-namespaces, one for bang macros and
// one for attribute-like macros (attributes, derives).
// We ignore resolutions from one sub-namespace when searching names in scope for another.
fn sub_namespace_match(candidate: Option<MacroKind>, requirement: Option<MacroKind>) -> bool {
    #[derive(PartialEq)]
    enum SubNS {
        Bang,
        AttrLike,
    }
    let sub_ns = |kind| match kind {
        MacroKind::Bang => SubNS::Bang,
        MacroKind::Attr | MacroKind::Derive => SubNS::AttrLike,
    };
    let candidate = candidate.map(sub_ns);
    let requirement = requirement.map(sub_ns);
    // "No specific sub-namespace" means "matches anything" for both requirements and candidates.
    candidate.is_none() || requirement.is_none() || candidate == requirement
}

// We don't want to format a path using pretty-printing,
// `format!("{}", path)`, because that tries to insert
// line-breaks and is slow.
fn fast_print_path(path: &ast::Path) -> Symbol {
    if path.segments.len() == 1 {
        path.segments[0].ident.name
    } else {
        let mut path_str = String::with_capacity(64);
        for (i, segment) in path.segments.iter().enumerate() {
            if i != 0 {
                path_str.push_str("::");
            }
            if segment.ident.name != kw::PathRoot {
                path_str.push_str(segment.ident.as_str())
            }
        }
        Symbol::intern(&path_str)
    }
}

/// The code common between processing `#![register_tool]` and `#![register_attr]`.
fn registered_idents(
    sess: &Session,
    attrs: &[ast::Attribute],
    attr_name: Symbol,
    descr: &str,
) -> FxHashSet<Ident> {
    let mut registered = FxHashSet::default();
    for attr in sess.filter_by_name(attrs, attr_name) {
        for nested_meta in attr.meta_item_list().unwrap_or_default() {
            match nested_meta.ident() {
                Some(ident) => {
                    if let Some(old_ident) = registered.replace(ident) {
                        let msg = format!("{} `{}` was already registered", descr, ident);
                        sess.struct_span_err(ident.span, &msg)
                            .span_label(old_ident.span, "already registered here")
                            .emit();
                    }
                }
                None => {
                    let msg = format!("`{}` only accepts identifiers", attr_name);
                    let span = nested_meta.span();
                    sess.struct_span_err(span, &msg).span_label(span, "not an identifier").emit();
                }
            }
        }
    }
    registered
}

crate fn registered_attrs_and_tools(
    sess: &Session,
    attrs: &[ast::Attribute],
) -> (FxHashSet<Ident>, FxHashSet<Ident>) {
    let registered_attrs = registered_idents(sess, attrs, sym::register_attr, "attribute");
    let mut registered_tools = registered_idents(sess, attrs, sym::register_tool, "tool");
    // We implicitly add `rustfmt` and `clippy` to known tools,
    // but it's not an error to register them explicitly.
    let predefined_tools = [sym::clippy, sym::rustfmt];
    registered_tools.extend(predefined_tools.iter().cloned().map(Ident::with_dummy_span));
    (registered_attrs, registered_tools)
}

// Some feature gates for inner attributes are reported as lints for backward compatibility.
fn soft_custom_inner_attributes_gate(path: &ast::Path, invoc: &Invocation) -> bool {
    match &path.segments[..] {
        // `#![test]`
        [seg] if seg.ident.name == sym::test => return true,
        // `#![rustfmt::skip]` on out-of-line modules
        [seg1, seg2] if seg1.ident.name == sym::rustfmt && seg2.ident.name == sym::skip => {
            if let InvocationKind::Attr { item, .. } = &invoc.kind {
                if let Annotatable::Item(item) = item {
                    if let ItemKind::Mod(_, ModKind::Loaded(_, Inline::No, _)) = item.kind {
                        return true;
                    }
                }
            }
        }
        _ => {}
    }
    false
}

impl<'a> ResolverExpand for Resolver<'a> {
    fn next_node_id(&mut self) -> NodeId {
        self.next_node_id()
    }

    fn invocation_parent(&self, id: LocalExpnId) -> LocalDefId {
        self.invocation_parents[&id].0
    }

    fn resolve_dollar_crates(&mut self) {
        hygiene::update_dollar_crate_names(|ctxt| {
            let ident = Ident::new(kw::DollarCrate, DUMMY_SP.with_ctxt(ctxt));
            match self.resolve_crate_root(ident).kind {
                ModuleKind::Def(.., name) if name != kw::Empty => name,
                _ => kw::Crate,
            }
        });
    }

    fn visit_ast_fragment_with_placeholders(
        &mut self,
        expansion: LocalExpnId,
        fragment: &AstFragment,
    ) {
        // Integrate the new AST fragment into all the definition and module structures.
        // We are inside the `expansion` now, but other parent scope components are still the same.
        let parent_scope = ParentScope { expansion, ..self.invocation_parent_scopes[&expansion] };
        let output_macro_rules_scope = self.build_reduced_graph(fragment, parent_scope);
        self.output_macro_rules_scopes.insert(expansion, output_macro_rules_scope);

        parent_scope.module.unexpanded_invocations.borrow_mut().remove(&expansion);
    }

    fn register_builtin_macro(&mut self, name: Symbol, ext: SyntaxExtensionKind) {
        if self.builtin_macros.insert(name, BuiltinMacroState::NotYetSeen(ext)).is_some() {
            self.session
                .diagnostic()
                .bug(&format!("built-in macro `{}` was already registered", name));
        }
    }

    // Create a new Expansion with a definition site of the provided module, or
    // a fake empty `#[no_implicit_prelude]` module if no module is provided.
    fn expansion_for_ast_pass(
        &mut self,
        call_site: Span,
        pass: AstPass,
        features: &[Symbol],
        parent_module_id: Option<NodeId>,
    ) -> LocalExpnId {
        let parent_module =
            parent_module_id.map(|module_id| self.local_def_id(module_id).to_def_id());
        let expn_id = LocalExpnId::fresh(
            ExpnData::allow_unstable(
                ExpnKind::AstPass(pass),
                call_site,
                self.session.edition(),
                features.into(),
                None,
                parent_module,
            ),
            self.create_stable_hashing_context(),
        );

        let parent_scope =
            parent_module.map_or(self.empty_module, |def_id| self.expect_module(def_id));
        self.ast_transform_scopes.insert(expn_id, parent_scope);

        expn_id
    }

    fn resolve_imports(&mut self) {
        ImportResolver { r: self }.resolve_imports()
    }

    fn resolve_macro_invocation(
        &mut self,
        invoc: &Invocation,
        eager_expansion_root: LocalExpnId,
        force: bool,
    ) -> Result<Lrc<SyntaxExtension>, Indeterminate> {
        let invoc_id = invoc.expansion_data.id;
        let parent_scope = match self.invocation_parent_scopes.get(&invoc_id) {
            Some(parent_scope) => *parent_scope,
            None => {
                // If there's no entry in the table, then we are resolving an eagerly expanded
                // macro, which should inherit its parent scope from its eager expansion root -
                // the macro that requested this eager expansion.
                let parent_scope = *self
                    .invocation_parent_scopes
                    .get(&eager_expansion_root)
                    .expect("non-eager expansion without a parent scope");
                self.invocation_parent_scopes.insert(invoc_id, parent_scope);
                parent_scope
            }
        };

        let (path, kind, inner_attr, derives) = match invoc.kind {
            InvocationKind::Attr { ref attr, ref derives, .. } => (
                &attr.get_normal_item().path,
                MacroKind::Attr,
                attr.style == ast::AttrStyle::Inner,
                self.arenas.alloc_ast_paths(derives),
            ),
            InvocationKind::Bang { ref mac, .. } => (&mac.path, MacroKind::Bang, false, &[][..]),
            InvocationKind::Derive { ref path, .. } => (path, MacroKind::Derive, false, &[][..]),
        };

        // Derives are not included when `invocations` are collected, so we have to add them here.
        let parent_scope = &ParentScope { derives, ..parent_scope };
        let supports_macro_expansion = invoc.fragment_kind.supports_macro_expansion();
        let node_id = invoc.expansion_data.lint_node_id;
        let (ext, res) = self.smart_resolve_macro_path(
            path,
            kind,
            supports_macro_expansion,
            inner_attr,
            parent_scope,
            node_id,
            force,
            soft_custom_inner_attributes_gate(path, invoc),
        )?;

        let span = invoc.span();
        let def_id = res.opt_def_id();
        invoc_id.set_expn_data(
            ext.expn_data(
                parent_scope.expansion,
                span,
                fast_print_path(path),
                def_id,
                def_id.map(|def_id| self.macro_def_scope(def_id).nearest_parent_mod()),
            ),
            self.create_stable_hashing_context(),
        );

        Ok(ext)
    }

    fn check_unused_macros(&mut self) {
        for (_, &(node_id, ident)) in self.unused_macros.iter() {
            self.lint_buffer.buffer_lint(
                UNUSED_MACROS,
                node_id,
                ident.span,
                &format!("unused macro definition: `{}`", ident.as_str()),
            );
        }
    }

    fn has_derive_copy(&self, expn_id: LocalExpnId) -> bool {
        self.containers_deriving_copy.contains(&expn_id)
    }

    fn resolve_derives(
        &mut self,
        expn_id: LocalExpnId,
        force: bool,
        derive_paths: &dyn Fn() -> DeriveResolutions,
    ) -> Result<(), Indeterminate> {
        // Block expansion of the container until we resolve all derives in it.
        // This is required for two reasons:
        // - Derive helper attributes are in scope for the item to which the `#[derive]`
        //   is applied, so they have to be produced by the container's expansion rather
        //   than by individual derives.
        // - Derives in the container need to know whether one of them is a built-in `Copy`.
        // Temporarily take the data to avoid borrow checker conflicts.
        let mut derive_data = mem::take(&mut self.derive_data);
        let entry = derive_data.entry(expn_id).or_insert_with(|| DeriveData {
            resolutions: derive_paths(),
            helper_attrs: Vec::new(),
            has_derive_copy: false,
        });
        let parent_scope = self.invocation_parent_scopes[&expn_id];
        for (i, (path, _, opt_ext)) in entry.resolutions.iter_mut().enumerate() {
            if opt_ext.is_none() {
                *opt_ext = Some(
                    match self.resolve_macro_path(
                        &path,
                        Some(MacroKind::Derive),
                        &parent_scope,
                        true,
                        force,
                    ) {
                        Ok((Some(ext), _)) => {
                            if !ext.helper_attrs.is_empty() {
                                let last_seg = path.segments.last().unwrap();
                                let span = last_seg.ident.span.normalize_to_macros_2_0();
                                entry.helper_attrs.extend(
                                    ext.helper_attrs
                                        .iter()
                                        .map(|name| (i, Ident::new(*name, span))),
                                );
                            }
                            entry.has_derive_copy |= ext.builtin_name == Some(sym::Copy);
                            ext
                        }
                        Ok(_) | Err(Determinacy::Determined) => self.dummy_ext(MacroKind::Derive),
                        Err(Determinacy::Undetermined) => {
                            assert!(self.derive_data.is_empty());
                            self.derive_data = derive_data;
                            return Err(Indeterminate);
                        }
                    },
                );
            }
        }
        // Sort helpers in a stable way independent from the derive resolution order.
        entry.helper_attrs.sort_by_key(|(i, _)| *i);
        self.helper_attrs
            .insert(expn_id, entry.helper_attrs.iter().map(|(_, ident)| *ident).collect());
        // Mark this derive as having `Copy` either if it has `Copy` itself or if its parent derive
        // has `Copy`, to support cases like `#[derive(Clone, Copy)] #[derive(Debug)]`.
        if entry.has_derive_copy || self.has_derive_copy(parent_scope.expansion) {
            self.containers_deriving_copy.insert(expn_id);
        }
        assert!(self.derive_data.is_empty());
        self.derive_data = derive_data;
        Ok(())
    }

    fn take_derive_resolutions(&mut self, expn_id: LocalExpnId) -> Option<DeriveResolutions> {
        self.derive_data.remove(&expn_id).map(|data| data.resolutions)
    }

    // The function that implements the resolution logic of `#[cfg_accessible(path)]`.
    // Returns true if the path can certainly be resolved in one of three namespaces,
    // returns false if the path certainly cannot be resolved in any of the three namespaces.
    // Returns `Indeterminate` if we cannot give a certain answer yet.
    fn cfg_accessible(
        &mut self,
        expn_id: LocalExpnId,
        path: &ast::Path,
    ) -> Result<bool, Indeterminate> {
        let span = path.span;
        let path = &Segment::from_path(path);
        let parent_scope = self.invocation_parent_scopes[&expn_id];

        let mut indeterminate = false;
        for ns in [TypeNS, ValueNS, MacroNS].iter().copied() {
            match self.resolve_path(path, Some(ns), &parent_scope, false, span, CrateLint::No) {
                PathResult::Module(ModuleOrUniformRoot::Module(_)) => return Ok(true),
                PathResult::NonModule(partial_res) if partial_res.unresolved_segments() == 0 => {
                    return Ok(true);
                }
                PathResult::Indeterminate => indeterminate = true,
                // FIXME: `resolve_path` is not ready to report partially resolved paths
                // correctly, so we just report an error if the path was reported as unresolved.
                // This needs to be fixed for `cfg_accessible` to be useful.
                PathResult::NonModule(..) | PathResult::Failed { .. } => {}
                PathResult::Module(_) => panic!("unexpected path resolution"),
            }
        }

        if indeterminate {
            return Err(Indeterminate);
        }

        self.session
            .struct_span_err(span, "not sure whether the path is accessible or not")
            .span_note(span, "`cfg_accessible` is not fully implemented")
            .emit();
        Ok(false)
    }

    fn get_proc_macro_quoted_span(&self, krate: CrateNum, id: usize) -> Span {
        self.crate_loader.cstore().get_proc_macro_quoted_span_untracked(krate, id, self.session)
    }

    fn declare_proc_macro(&mut self, id: NodeId) {
        self.proc_macros.push(id)
    }
}

impl<'a> Resolver<'a> {
    /// Resolve macro path with error reporting and recovery.
    /// Uses dummy syntax extensions for unresolved macros or macros with unexpected resolutions
    /// for better error recovery.
    fn smart_resolve_macro_path(
        &mut self,
        path: &ast::Path,
        kind: MacroKind,
        supports_macro_expansion: SupportsMacroExpansion,
        inner_attr: bool,
        parent_scope: &ParentScope<'a>,
        node_id: NodeId,
        force: bool,
        soft_custom_inner_attributes_gate: bool,
    ) -> Result<(Lrc<SyntaxExtension>, Res), Indeterminate> {
        let (ext, res) = match self.resolve_macro_path(path, Some(kind), parent_scope, true, force)
        {
            Ok((Some(ext), res)) => (ext, res),
            Ok((None, res)) => (self.dummy_ext(kind), res),
            Err(Determinacy::Determined) => (self.dummy_ext(kind), Res::Err),
            Err(Determinacy::Undetermined) => return Err(Indeterminate),
        };

        // Report errors for the resolved macro.
        for segment in &path.segments {
            if let Some(args) = &segment.args {
                self.session.span_err(args.span(), "generic arguments in macro path");
            }
            if kind == MacroKind::Attr && segment.ident.as_str().starts_with("rustc") {
                self.session.span_err(
                    segment.ident.span,
                    "attributes starting with `rustc` are reserved for use by the `rustc` compiler",
                );
            }
        }

        match res {
            Res::Def(DefKind::Macro(_), def_id) => {
                if let Some(def_id) = def_id.as_local() {
                    self.unused_macros.remove(&def_id);
                    if self.proc_macro_stubs.contains(&def_id) {
                        self.session.span_err(
                            path.span,
                            "can't use a procedural macro from the same crate that defines it",
                        );
                    }
                }
            }
            Res::NonMacroAttr(..) | Res::Err => {}
            _ => panic!("expected `DefKind::Macro` or `Res::NonMacroAttr`"),
        };

        self.check_stability_and_deprecation(&ext, path, node_id);

        let unexpected_res = if ext.macro_kind() != kind {
            Some((kind.article(), kind.descr_expected()))
        } else if matches!(res, Res::Def(..)) {
            match supports_macro_expansion {
                SupportsMacroExpansion::No => Some(("a", "non-macro attribute")),
                SupportsMacroExpansion::Yes { supports_inner_attrs } => {
                    if inner_attr && !supports_inner_attrs {
                        Some(("a", "non-macro inner attribute"))
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        };
        if let Some((article, expected)) = unexpected_res {
            let path_str = pprust::path_to_string(path);
            let msg = format!("expected {}, found {} `{}`", expected, res.descr(), path_str);
            self.session
                .struct_span_err(path.span, &msg)
                .span_label(path.span, format!("not {} {}", article, expected))
                .emit();
            return Ok((self.dummy_ext(kind), Res::Err));
        }

        // We are trying to avoid reporting this error if other related errors were reported.
        if res != Res::Err
            && inner_attr
            && !self.session.features_untracked().custom_inner_attributes
        {
            let msg = match res {
                Res::Def(..) => "inner macro attributes are unstable",
                Res::NonMacroAttr(..) => "custom inner attributes are unstable",
                _ => unreachable!(),
            };
            if soft_custom_inner_attributes_gate {
                self.session.parse_sess.buffer_lint(SOFT_UNSTABLE, path.span, node_id, msg);
            } else {
                feature_err(&self.session.parse_sess, sym::custom_inner_attributes, path.span, msg)
                    .emit();
            }
        }

        Ok((ext, res))
    }

    pub fn resolve_macro_path(
        &mut self,
        path: &ast::Path,
        kind: Option<MacroKind>,
        parent_scope: &ParentScope<'a>,
        trace: bool,
        force: bool,
    ) -> Result<(Option<Lrc<SyntaxExtension>>, Res), Determinacy> {
        let path_span = path.span;
        let mut path = Segment::from_path(path);

        // Possibly apply the macro helper hack
        if kind == Some(MacroKind::Bang)
            && path.len() == 1
            && path[0].ident.span.ctxt().outer_expn_data().local_inner_macros
        {
            let root = Ident::new(kw::DollarCrate, path[0].ident.span);
            path.insert(0, Segment::from_ident(root));
        }

        let res = if path.len() > 1 {
            let res = match self.resolve_path(
                &path,
                Some(MacroNS),
                parent_scope,
                false,
                path_span,
                CrateLint::No,
            ) {
                PathResult::NonModule(path_res) if path_res.unresolved_segments() == 0 => {
                    Ok(path_res.base_res())
                }
                PathResult::Indeterminate if !force => return Err(Determinacy::Undetermined),
                PathResult::NonModule(..)
                | PathResult::Indeterminate
                | PathResult::Failed { .. } => Err(Determinacy::Determined),
                PathResult::Module(..) => unreachable!(),
            };

            if trace {
                let kind = kind.expect("macro kind must be specified if tracing is enabled");
                self.multi_segment_macro_resolutions.push((
                    path,
                    path_span,
                    kind,
                    *parent_scope,
                    res.ok(),
                ));
            }

            self.prohibit_imported_non_macro_attrs(None, res.ok(), path_span);
            res
        } else {
            let scope_set = kind.map_or(ScopeSet::All(MacroNS, false), ScopeSet::Macro);
            let binding = self.early_resolve_ident_in_lexical_scope(
                path[0].ident,
                scope_set,
                parent_scope,
                false,
                force,
                path_span,
            );
            if let Err(Determinacy::Undetermined) = binding {
                return Err(Determinacy::Undetermined);
            }

            if trace {
                let kind = kind.expect("macro kind must be specified if tracing is enabled");
                self.single_segment_macro_resolutions.push((
                    path[0].ident,
                    kind,
                    *parent_scope,
                    binding.ok(),
                ));
            }

            let res = binding.map(|binding| binding.res());
            self.prohibit_imported_non_macro_attrs(binding.ok(), res.ok(), path_span);
            res
        };

        res.map(|res| (self.get_macro(res), res))
    }

    // Resolve an identifier in lexical scope.
    // This is a variation of `fn resolve_ident_in_lexical_scope` that can be run during
    // expansion and import resolution (perhaps they can be merged in the future).
    // The function is used for resolving initial segments of macro paths (e.g., `foo` in
    // `foo::bar!(); or `foo!();`) and also for import paths on 2018 edition.
    crate fn early_resolve_ident_in_lexical_scope(
        &mut self,
        orig_ident: Ident,
        scope_set: ScopeSet<'a>,
        parent_scope: &ParentScope<'a>,
        record_used: bool,
        force: bool,
        path_span: Span,
    ) -> Result<&'a NameBinding<'a>, Determinacy> {
        bitflags::bitflags! {
            struct Flags: u8 {
                const MACRO_RULES          = 1 << 0;
                const MODULE               = 1 << 1;
                const MISC_SUGGEST_CRATE   = 1 << 2;
                const MISC_SUGGEST_SELF    = 1 << 3;
                const MISC_FROM_PRELUDE    = 1 << 4;
            }
        }

        assert!(force || !record_used); // `record_used` implies `force`

        // Make sure `self`, `super` etc produce an error when passed to here.
        if orig_ident.is_path_segment_keyword() {
            return Err(Determinacy::Determined);
        }

        let (ns, macro_kind, is_import) = match scope_set {
            ScopeSet::All(ns, is_import) => (ns, None, is_import),
            ScopeSet::AbsolutePath(ns) => (ns, None, false),
            ScopeSet::Macro(macro_kind) => (MacroNS, Some(macro_kind), false),
            ScopeSet::Late(ns, ..) => (ns, None, false),
        };

        // This is *the* result, resolution from the scope closest to the resolved identifier.
        // However, sometimes this result is "weak" because it comes from a glob import or
        // a macro expansion, and in this case it cannot shadow names from outer scopes, e.g.
        // mod m { ... } // solution in outer scope
        // {
        //     use prefix::*; // imports another `m` - innermost solution
        //                    // weak, cannot shadow the outer `m`, need to report ambiguity error
        //     m::mac!();
        // }
        // So we have to save the innermost solution and continue searching in outer scopes
        // to detect potential ambiguities.
        let mut innermost_result: Option<(&NameBinding<'_>, Flags)> = None;
        let mut determinacy = Determinacy::Determined;

        // Go through all the scopes and try to resolve the name.
        let break_result = self.visit_scopes(
            scope_set,
            parent_scope,
            orig_ident.span.ctxt(),
            |this, scope, use_prelude, ctxt| {
                let ident = Ident::new(orig_ident.name, orig_ident.span.with_ctxt(ctxt));
                let ok = |res, span, arenas| {
                    Ok((
                        (res, ty::Visibility::Public, span, LocalExpnId::ROOT)
                            .to_name_binding(arenas),
                        Flags::empty(),
                    ))
                };
                let result = match scope {
                    Scope::DeriveHelpers(expn_id) => {
                        if let Some(attr) = this
                            .helper_attrs
                            .get(&expn_id)
                            .and_then(|attrs| attrs.iter().rfind(|i| ident == **i))
                        {
                            let binding = (
                                Res::NonMacroAttr(NonMacroAttrKind::DeriveHelper),
                                ty::Visibility::Public,
                                attr.span,
                                expn_id,
                            )
                                .to_name_binding(this.arenas);
                            Ok((binding, Flags::empty()))
                        } else {
                            Err(Determinacy::Determined)
                        }
                    }
                    Scope::DeriveHelpersCompat => {
                        let mut result = Err(Determinacy::Determined);
                        for derive in parent_scope.derives {
                            let parent_scope = &ParentScope { derives: &[], ..*parent_scope };
                            match this.resolve_macro_path(
                                derive,
                                Some(MacroKind::Derive),
                                parent_scope,
                                true,
                                force,
                            ) {
                                Ok((Some(ext), _)) => {
                                    if ext.helper_attrs.contains(&ident.name) {
                                        result = ok(
                                            Res::NonMacroAttr(NonMacroAttrKind::DeriveHelperCompat),
                                            derive.span,
                                            this.arenas,
                                        );
                                        break;
                                    }
                                }
                                Ok(_) | Err(Determinacy::Determined) => {}
                                Err(Determinacy::Undetermined) => {
                                    result = Err(Determinacy::Undetermined)
                                }
                            }
                        }
                        result
                    }
                    Scope::MacroRules(macro_rules_scope) => match macro_rules_scope.get() {
                        MacroRulesScope::Binding(macro_rules_binding)
                            if ident == macro_rules_binding.ident =>
                        {
                            Ok((macro_rules_binding.binding, Flags::MACRO_RULES))
                        }
                        MacroRulesScope::Invocation(_) => Err(Determinacy::Undetermined),
                        _ => Err(Determinacy::Determined),
                    },
                    Scope::CrateRoot => {
                        let root_ident = Ident::new(kw::PathRoot, ident.span);
                        let root_module = this.resolve_crate_root(root_ident);
                        let binding = this.resolve_ident_in_module_ext(
                            ModuleOrUniformRoot::Module(root_module),
                            ident,
                            ns,
                            parent_scope,
                            record_used,
                            path_span,
                        );
                        match binding {
                            Ok(binding) => Ok((binding, Flags::MODULE | Flags::MISC_SUGGEST_CRATE)),
                            Err((Determinacy::Undetermined, Weak::No)) => {
                                return Some(Err(Determinacy::determined(force)));
                            }
                            Err((Determinacy::Undetermined, Weak::Yes)) => {
                                Err(Determinacy::Undetermined)
                            }
                            Err((Determinacy::Determined, _)) => Err(Determinacy::Determined),
                        }
                    }
                    Scope::Module(module, derive_fallback_lint_id) => {
                        let adjusted_parent_scope = &ParentScope { module, ..*parent_scope };
                        let binding = this.resolve_ident_in_module_unadjusted_ext(
                            ModuleOrUniformRoot::Module(module),
                            ident,
                            ns,
                            adjusted_parent_scope,
                            !matches!(scope_set, ScopeSet::Late(..)),
                            record_used,
                            path_span,
                        );
                        match binding {
                            Ok(binding) => {
                                if let Some(lint_id) = derive_fallback_lint_id {
                                    this.lint_buffer.buffer_lint_with_diagnostic(
                                        PROC_MACRO_DERIVE_RESOLUTION_FALLBACK,
                                        lint_id,
                                        orig_ident.span,
                                        &format!(
                                            "cannot find {} `{}` in this scope",
                                            ns.descr(),
                                            ident
                                        ),
                                        BuiltinLintDiagnostics::ProcMacroDeriveResolutionFallback(
                                            orig_ident.span,
                                        ),
                                    );
                                }
                                let misc_flags = if ptr::eq(module, this.graph_root) {
                                    Flags::MISC_SUGGEST_CRATE
                                } else if module.is_normal() {
                                    Flags::MISC_SUGGEST_SELF
                                } else {
                                    Flags::empty()
                                };
                                Ok((binding, Flags::MODULE | misc_flags))
                            }
                            Err((Determinacy::Undetermined, Weak::No)) => {
                                return Some(Err(Determinacy::determined(force)));
                            }
                            Err((Determinacy::Undetermined, Weak::Yes)) => {
                                Err(Determinacy::Undetermined)
                            }
                            Err((Determinacy::Determined, _)) => Err(Determinacy::Determined),
                        }
                    }
                    Scope::RegisteredAttrs => match this.registered_attrs.get(&ident).cloned() {
                        Some(ident) => ok(
                            Res::NonMacroAttr(NonMacroAttrKind::Registered),
                            ident.span,
                            this.arenas,
                        ),
                        None => Err(Determinacy::Determined),
                    },
                    Scope::MacroUsePrelude => {
                        match this.macro_use_prelude.get(&ident.name).cloned() {
                            Some(binding) => Ok((binding, Flags::MISC_FROM_PRELUDE)),
                            None => Err(Determinacy::determined(
                                this.graph_root.unexpanded_invocations.borrow().is_empty(),
                            )),
                        }
                    }
                    Scope::BuiltinAttrs => {
                        if is_builtin_attr_name(ident.name) {
                            ok(
                                Res::NonMacroAttr(NonMacroAttrKind::Builtin(ident.name)),
                                DUMMY_SP,
                                this.arenas,
                            )
                        } else {
                            Err(Determinacy::Determined)
                        }
                    }
                    Scope::ExternPrelude => match this.extern_prelude_get(ident, !record_used) {
                        Some(binding) => Ok((binding, Flags::empty())),
                        None => Err(Determinacy::determined(
                            this.graph_root.unexpanded_invocations.borrow().is_empty(),
                        )),
                    },
                    Scope::ToolPrelude => match this.registered_tools.get(&ident).cloned() {
                        Some(ident) => ok(Res::ToolMod, ident.span, this.arenas),
                        None => Err(Determinacy::Determined),
                    },
                    Scope::StdLibPrelude => {
                        let mut result = Err(Determinacy::Determined);
                        if let Some(prelude) = this.prelude {
                            if let Ok(binding) = this.resolve_ident_in_module_unadjusted(
                                ModuleOrUniformRoot::Module(prelude),
                                ident,
                                ns,
                                parent_scope,
                                false,
                                path_span,
                            ) {
                                if use_prelude || this.is_builtin_macro(binding.res()) {
                                    result = Ok((binding, Flags::MISC_FROM_PRELUDE));
                                }
                            }
                        }
                        result
                    }
                    Scope::BuiltinTypes => match PrimTy::from_name(ident.name) {
                        Some(prim_ty) => ok(Res::PrimTy(prim_ty), DUMMY_SP, this.arenas),
                        None => Err(Determinacy::Determined),
                    },
                };

                match result {
                    Ok((binding, flags))
                        if sub_namespace_match(binding.macro_kind(), macro_kind) =>
                    {
                        if !record_used || matches!(scope_set, ScopeSet::Late(..)) {
                            return Some(Ok(binding));
                        }

                        if let Some((innermost_binding, innermost_flags)) = innermost_result {
                            // Found another solution, if the first one was "weak", report an error.
                            let (res, innermost_res) = (binding.res(), innermost_binding.res());
                            if res != innermost_res {
                                let is_builtin = |res| {
                                    matches!(res, Res::NonMacroAttr(NonMacroAttrKind::Builtin(..)))
                                };
                                let derive_helper =
                                    Res::NonMacroAttr(NonMacroAttrKind::DeriveHelper);
                                let derive_helper_compat =
                                    Res::NonMacroAttr(NonMacroAttrKind::DeriveHelperCompat);

                                let ambiguity_error_kind = if is_import {
                                    Some(AmbiguityKind::Import)
                                } else if is_builtin(innermost_res) || is_builtin(res) {
                                    Some(AmbiguityKind::BuiltinAttr)
                                } else if innermost_res == derive_helper_compat
                                    || res == derive_helper_compat && innermost_res != derive_helper
                                {
                                    Some(AmbiguityKind::DeriveHelper)
                                } else if innermost_flags.contains(Flags::MACRO_RULES)
                                    && flags.contains(Flags::MODULE)
                                    && !this.disambiguate_macro_rules_vs_modularized(
                                        innermost_binding,
                                        binding,
                                    )
                                    || flags.contains(Flags::MACRO_RULES)
                                        && innermost_flags.contains(Flags::MODULE)
                                        && !this.disambiguate_macro_rules_vs_modularized(
                                            binding,
                                            innermost_binding,
                                        )
                                {
                                    Some(AmbiguityKind::MacroRulesVsModularized)
                                } else if innermost_binding.is_glob_import() {
                                    Some(AmbiguityKind::GlobVsOuter)
                                } else if innermost_binding
                                    .may_appear_after(parent_scope.expansion, binding)
                                {
                                    Some(AmbiguityKind::MoreExpandedVsOuter)
                                } else {
                                    None
                                };
                                if let Some(kind) = ambiguity_error_kind {
                                    let misc = |f: Flags| {
                                        if f.contains(Flags::MISC_SUGGEST_CRATE) {
                                            AmbiguityErrorMisc::SuggestCrate
                                        } else if f.contains(Flags::MISC_SUGGEST_SELF) {
                                            AmbiguityErrorMisc::SuggestSelf
                                        } else if f.contains(Flags::MISC_FROM_PRELUDE) {
                                            AmbiguityErrorMisc::FromPrelude
                                        } else {
                                            AmbiguityErrorMisc::None
                                        }
                                    };
                                    this.ambiguity_errors.push(AmbiguityError {
                                        kind,
                                        ident: orig_ident,
                                        b1: innermost_binding,
                                        b2: binding,
                                        misc1: misc(innermost_flags),
                                        misc2: misc(flags),
                                    });
                                    return Some(Ok(innermost_binding));
                                }
                            }
                        } else {
                            // Found the first solution.
                            innermost_result = Some((binding, flags));
                        }
                    }
                    Ok(..) | Err(Determinacy::Determined) => {}
                    Err(Determinacy::Undetermined) => determinacy = Determinacy::Undetermined,
                }

                None
            },
        );

        if let Some(break_result) = break_result {
            return break_result;
        }

        // The first found solution was the only one, return it.
        if let Some((binding, _)) = innermost_result {
            return Ok(binding);
        }

        Err(Determinacy::determined(determinacy == Determinacy::Determined || force))
    }

    crate fn finalize_macro_resolutions(&mut self) {
        let check_consistency = |this: &mut Self,
                                 path: &[Segment],
                                 span,
                                 kind: MacroKind,
                                 initial_res: Option<Res>,
                                 res: Res| {
            if let Some(initial_res) = initial_res {
                if res != initial_res {
                    // Make sure compilation does not succeed if preferred macro resolution
                    // has changed after the macro had been expanded. In theory all such
                    // situations should be reported as errors, so this is a bug.
                    this.session.delay_span_bug(span, "inconsistent resolution for a macro");
                }
            } else {
                // It's possible that the macro was unresolved (indeterminate) and silently
                // expanded into a dummy fragment for recovery during expansion.
                // Now, post-expansion, the resolution may succeed, but we can't change the
                // past and need to report an error.
                // However, non-speculative `resolve_path` can successfully return private items
                // even if speculative `resolve_path` returned nothing previously, so we skip this
                // less informative error if the privacy error is reported elsewhere.
                if this.privacy_errors.is_empty() {
                    let msg = format!(
                        "cannot determine resolution for the {} `{}`",
                        kind.descr(),
                        Segment::names_to_string(path)
                    );
                    let msg_note = "import resolution is stuck, try simplifying macro imports";
                    this.session.struct_span_err(span, &msg).note(msg_note).emit();
                }
            }
        };

        let macro_resolutions = mem::take(&mut self.multi_segment_macro_resolutions);
        for (mut path, path_span, kind, parent_scope, initial_res) in macro_resolutions {
            // FIXME: Path resolution will ICE if segment IDs present.
            for seg in &mut path {
                seg.id = None;
            }
            match self.resolve_path(
                &path,
                Some(MacroNS),
                &parent_scope,
                true,
                path_span,
                CrateLint::No,
            ) {
                PathResult::NonModule(path_res) if path_res.unresolved_segments() == 0 => {
                    let res = path_res.base_res();
                    check_consistency(self, &path, path_span, kind, initial_res, res);
                }
                path_res @ PathResult::NonModule(..) | path_res @ PathResult::Failed { .. } => {
                    let (span, label) = if let PathResult::Failed { span, label, .. } = path_res {
                        (span, label)
                    } else {
                        (
                            path_span,
                            format!(
                                "partially resolved path in {} {}",
                                kind.article(),
                                kind.descr()
                            ),
                        )
                    };
                    self.report_error(
                        span,
                        ResolutionError::FailedToResolve { label, suggestion: None },
                    );
                }
                PathResult::Module(..) | PathResult::Indeterminate => unreachable!(),
            }
        }

        let macro_resolutions = mem::take(&mut self.single_segment_macro_resolutions);
        for (ident, kind, parent_scope, initial_binding) in macro_resolutions {
            match self.early_resolve_ident_in_lexical_scope(
                ident,
                ScopeSet::Macro(kind),
                &parent_scope,
                true,
                true,
                ident.span,
            ) {
                Ok(binding) => {
                    let initial_res = initial_binding.map(|initial_binding| {
                        self.record_use(ident, initial_binding, false);
                        initial_binding.res()
                    });
                    let res = binding.res();
                    let seg = Segment::from_ident(ident);
                    check_consistency(self, &[seg], ident.span, kind, initial_res, res);
                    if res == Res::NonMacroAttr(NonMacroAttrKind::DeriveHelperCompat) {
                        let node_id = self
                            .invocation_parents
                            .get(&parent_scope.expansion)
                            .map_or(ast::CRATE_NODE_ID, |id| self.def_id_to_node_id[id.0]);
                        self.lint_buffer.buffer_lint_with_diagnostic(
                            LEGACY_DERIVE_HELPERS,
                            node_id,
                            ident.span,
                            "derive helper attribute is used before it is introduced",
                            BuiltinLintDiagnostics::LegacyDeriveHelpers(binding.span),
                        );
                    }
                }
                Err(..) => {
                    let expected = kind.descr_expected();
                    let msg = format!("cannot find {} `{}` in this scope", expected, ident);
                    let mut err = self.session.struct_span_err(ident.span, &msg);
                    self.unresolved_macro_suggestions(&mut err, kind, &parent_scope, ident);
                    err.emit();
                }
            }
        }

        let builtin_attrs = mem::take(&mut self.builtin_attrs);
        for (ident, parent_scope) in builtin_attrs {
            let _ = self.early_resolve_ident_in_lexical_scope(
                ident,
                ScopeSet::Macro(MacroKind::Attr),
                &parent_scope,
                true,
                true,
                ident.span,
            );
        }
    }

    fn check_stability_and_deprecation(
        &mut self,
        ext: &SyntaxExtension,
        path: &ast::Path,
        node_id: NodeId,
    ) {
        let span = path.span;
        if let Some(stability) = &ext.stability {
            if let StabilityLevel::Unstable { reason, issue, is_soft } = stability.level {
                let feature = stability.feature;
                if !self.active_features.contains(&feature) && !span.allows_unstable(feature) {
                    let lint_buffer = &mut self.lint_buffer;
                    let soft_handler =
                        |lint, span, msg: &_| lint_buffer.buffer_lint(lint, node_id, span, msg);
                    stability::report_unstable(
                        self.session,
                        feature,
                        reason,
                        issue,
                        None,
                        is_soft,
                        span,
                        soft_handler,
                    );
                }
            }
        }
        if let Some(depr) = &ext.deprecation {
            let path = pprust::path_to_string(&path);
            let (message, lint) = stability::deprecation_message_and_lint(depr, "macro", &path);
            stability::early_report_deprecation(
                &mut self.lint_buffer,
                &message,
                depr.suggestion,
                lint,
                span,
                node_id,
            );
        }
    }

    fn prohibit_imported_non_macro_attrs(
        &self,
        binding: Option<&'a NameBinding<'a>>,
        res: Option<Res>,
        span: Span,
    ) {
        if let Some(Res::NonMacroAttr(kind)) = res {
            if kind != NonMacroAttrKind::Tool && binding.map_or(true, |b| b.is_import()) {
                let msg =
                    format!("cannot use {} {} through an import", kind.article(), kind.descr());
                let mut err = self.session.struct_span_err(span, &msg);
                if let Some(binding) = binding {
                    err.span_note(binding.span, &format!("the {} imported here", kind.descr()));
                }
                err.emit();
            }
        }
    }

    crate fn check_reserved_macro_name(&mut self, ident: Ident, res: Res) {
        // Reserve some names that are not quite covered by the general check
        // performed on `Resolver::builtin_attrs`.
        if ident.name == sym::cfg || ident.name == sym::cfg_attr {
            let macro_kind = self.get_macro(res).map(|ext| ext.macro_kind());
            if macro_kind.is_some() && sub_namespace_match(macro_kind, Some(MacroKind::Attr)) {
                self.session.span_err(
                    ident.span,
                    &format!("name `{}` is reserved in attribute namespace", ident),
                );
            }
        }
    }

    /// Compile the macro into a `SyntaxExtension` and possibly replace
    /// its expander to a pre-defined one for built-in macros.
    crate fn compile_macro(&mut self, item: &ast::Item, edition: Edition) -> SyntaxExtension {
        let mut result = compile_declarative_macro(
            &self.session,
            self.session.features_untracked(),
            item,
            edition,
        );

        if let Some(builtin_name) = result.builtin_name {
            // The macro was marked with `#[rustc_builtin_macro]`.
            if let Some(builtin_macro) = self.builtin_macros.get_mut(&builtin_name) {
                // The macro is a built-in, replace its expander function
                // while still taking everything else from the source code.
                // If we already loaded this builtin macro, give a better error message than 'no such builtin macro'.
                match mem::replace(builtin_macro, BuiltinMacroState::AlreadySeen(item.span)) {
                    BuiltinMacroState::NotYetSeen(ext) => result.kind = ext,
                    BuiltinMacroState::AlreadySeen(span) => {
                        struct_span_err!(
                            self.session,
                            item.span,
                            E0773,
                            "attempted to define built-in macro more than once"
                        )
                        .span_note(span, "previously defined here")
                        .emit();
                    }
                }
            } else {
                let msg = format!("cannot find a built-in macro with name `{}`", item.ident);
                self.session.span_err(item.span, &msg);
            }
        }

        result
    }
}
