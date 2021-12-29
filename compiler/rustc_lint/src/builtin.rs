//! Lints in the Rust compiler.
//!
//! This contains lints which can feasibly be implemented as their own
//! AST visitor. Also see `rustc_session::lint::builtin`, which contains the
//! definitions of lints that are emitted directly inside the main compiler.
//!
//! To add a new lint to rustc, declare it here using `declare_lint!()`.
//! Then add code to emit the new lint in the appropriate circumstances.
//! You can do that in an existing `LintPass` if it makes sense, or in a
//! new `LintPass`, or using `Session::add_lint` elsewhere in the
//! compiler. Only do the latter if the check can't be written cleanly as a
//! `LintPass` (also, note that such lints will need to be defined in
//! `rustc_session::lint::builtin`, not here).
//!
//! If you define a new `EarlyLintPass`, you will also need to add it to the
//! `add_early_builtin!` or `add_early_builtin_with_new!` invocation in
//! `lib.rs`. Use the former for unit-like structs and the latter for structs
//! with a `pub fn new()`.
//!
//! If you define a new `LateLintPass`, you will also need to add it to the
//! `late_lint_methods!` invocation in `lib.rs`.

use crate::{
    types::{transparent_newtype_field, CItemKind},
    EarlyContext, EarlyLintPass, LateContext, LateLintPass, LintContext,
};
use rustc_ast::attr;
use rustc_ast::tokenstream::{TokenStream, TokenTree};
use rustc_ast::visit::{FnCtxt, FnKind};
use rustc_ast::{self as ast, *};
use rustc_ast_pretty::pprust::{self, expr_to_string};
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_data_structures::stack::ensure_sufficient_stack;
use rustc_errors::{Applicability, DiagnosticBuilder, DiagnosticStyledString};
use rustc_feature::{deprecated_attributes, AttributeGate, BuiltinAttribute, GateIssue, Stability};
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId, LocalDefIdSet, CRATE_DEF_ID};
use rustc_hir::{ForeignItemKind, GenericParamKind, PatKind};
use rustc_hir::{HirId, Node};
use rustc_index::vec::Idx;
use rustc_middle::lint::LintDiagnosticBuilder;
use rustc_middle::ty::layout::{LayoutError, LayoutOf};
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::subst::{GenericArgKind, Subst};
use rustc_middle::ty::Instance;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_session::lint::{BuiltinLintDiagnostics, FutureIncompatibilityReason};
use rustc_span::edition::Edition;
use rustc_span::source_map::Spanned;
use rustc_span::symbol::{kw, sym, Ident, Symbol};
use rustc_span::{BytePos, InnerSpan, MultiSpan, Span};
use rustc_target::abi::VariantIdx;
use rustc_trait_selection::traits::misc::can_type_implement_copy;

use crate::nonstandard_style::{method_context, MethodLateContext};

use std::fmt::Write;
use tracing::{debug, trace};

// hardwired lints from librustc_middle
pub use rustc_session::lint::builtin::*;

declare_lint! {
    /// The `while_true` lint detects `while true { }`.
    ///
    /// ### Example
    ///
    /// ```rust,no_run
    /// while true {
    ///
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// `while true` should be replaced with `loop`. A `loop` expression is
    /// the preferred way to write an infinite loop because it more directly
    /// expresses the intent of the loop.
    WHILE_TRUE,
    Warn,
    "suggest using `loop { }` instead of `while true { }`"
}

declare_lint_pass!(WhileTrue => [WHILE_TRUE]);

/// Traverse through any amount of parenthesis and return the first non-parens expression.
fn pierce_parens(mut expr: &ast::Expr) -> &ast::Expr {
    while let ast::ExprKind::Paren(sub) = &expr.kind {
        expr = sub;
    }
    expr
}

impl EarlyLintPass for WhileTrue {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, e: &ast::Expr) {
        if let ast::ExprKind::While(cond, _, label) = &e.kind {
            if let ast::ExprKind::Lit(ref lit) = pierce_parens(cond).kind {
                if let ast::LitKind::Bool(true) = lit.kind {
                    if !lit.span.from_expansion() {
                        let msg = "denote infinite loops with `loop { ... }`";
                        let condition_span = e.span.with_hi(cond.span.hi());
                        cx.struct_span_lint(WHILE_TRUE, condition_span, |lint| {
                            lint.build(msg)
                                .span_suggestion_short(
                                    condition_span,
                                    "use `loop`",
                                    format!(
                                        "{}loop",
                                        label.map_or_else(String::new, |label| format!(
                                            "{}: ",
                                            label.ident,
                                        ))
                                    ),
                                    Applicability::MachineApplicable,
                                )
                                .emit();
                        })
                    }
                }
            }
        }
    }
}

declare_lint! {
    /// The `box_pointers` lints use of the Box type.
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// #![deny(box_pointers)]
    /// struct Foo {
    ///     x: Box<isize>,
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// This lint is mostly historical, and not particularly useful. `Box<T>`
    /// used to be built into the language, and the only way to do heap
    /// allocation. Today's Rust can call into other allocators, etc.
    BOX_POINTERS,
    Allow,
    "use of owned (Box type) heap memory"
}

declare_lint_pass!(BoxPointers => [BOX_POINTERS]);

impl BoxPointers {
    fn check_heap_type<'tcx>(&self, cx: &LateContext<'tcx>, span: Span, ty: Ty<'tcx>) {
        for leaf in ty.walk(cx.tcx) {
            if let GenericArgKind::Type(leaf_ty) = leaf.unpack() {
                if leaf_ty.is_box() {
                    cx.struct_span_lint(BOX_POINTERS, span, |lint| {
                        lint.build(&format!("type uses owned (Box type) pointers: {}", ty)).emit()
                    });
                }
            }
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for BoxPointers {
    fn check_item(&mut self, cx: &LateContext<'_>, it: &hir::Item<'_>) {
        match it.kind {
            hir::ItemKind::Fn(..)
            | hir::ItemKind::TyAlias(..)
            | hir::ItemKind::Enum(..)
            | hir::ItemKind::Struct(..)
            | hir::ItemKind::Union(..) => {
                self.check_heap_type(cx, it.span, cx.tcx.type_of(it.def_id))
            }
            _ => (),
        }

        // If it's a struct, we also have to check the fields' types
        match it.kind {
            hir::ItemKind::Struct(ref struct_def, _) | hir::ItemKind::Union(ref struct_def, _) => {
                for struct_field in struct_def.fields() {
                    let def_id = cx.tcx.hir().local_def_id(struct_field.hir_id);
                    self.check_heap_type(cx, struct_field.span, cx.tcx.type_of(def_id));
                }
            }
            _ => (),
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'_>, e: &hir::Expr<'_>) {
        let ty = cx.typeck_results().node_type(e.hir_id);
        self.check_heap_type(cx, e.span, ty);
    }
}

declare_lint! {
    /// The `non_shorthand_field_patterns` lint detects using `Struct { x: x }`
    /// instead of `Struct { x }` in a pattern.
    ///
    /// ### Example
    ///
    /// ```rust
    /// struct Point {
    ///     x: i32,
    ///     y: i32,
    /// }
    ///
    ///
    /// fn main() {
    ///     let p = Point {
    ///         x: 5,
    ///         y: 5,
    ///     };
    ///
    ///     match p {
    ///         Point { x: x, y: y } => (),
    ///     }
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// The preferred style is to avoid the repetition of specifying both the
    /// field name and the binding name if both identifiers are the same.
    NON_SHORTHAND_FIELD_PATTERNS,
    Warn,
    "using `Struct { x: x }` instead of `Struct { x }` in a pattern"
}

declare_lint_pass!(NonShorthandFieldPatterns => [NON_SHORTHAND_FIELD_PATTERNS]);

impl<'tcx> LateLintPass<'tcx> for NonShorthandFieldPatterns {
    fn check_pat(&mut self, cx: &LateContext<'_>, pat: &hir::Pat<'_>) {
        if let PatKind::Struct(ref qpath, field_pats, _) = pat.kind {
            let variant = cx
                .typeck_results()
                .pat_ty(pat)
                .ty_adt_def()
                .expect("struct pattern type is not an ADT")
                .variant_of_res(cx.qpath_res(qpath, pat.hir_id));
            for fieldpat in field_pats {
                if fieldpat.is_shorthand {
                    continue;
                }
                if fieldpat.span.from_expansion() {
                    // Don't lint if this is a macro expansion: macro authors
                    // shouldn't have to worry about this kind of style issue
                    // (Issue #49588)
                    continue;
                }
                if let PatKind::Binding(binding_annot, _, ident, None) = fieldpat.pat.kind {
                    if cx.tcx.find_field_index(ident, &variant)
                        == Some(cx.tcx.field_index(fieldpat.hir_id, cx.typeck_results()))
                    {
                        cx.struct_span_lint(NON_SHORTHAND_FIELD_PATTERNS, fieldpat.span, |lint| {
                            let mut err = lint
                                .build(&format!("the `{}:` in this pattern is redundant", ident));
                            let binding = match binding_annot {
                                hir::BindingAnnotation::Unannotated => None,
                                hir::BindingAnnotation::Mutable => Some("mut"),
                                hir::BindingAnnotation::Ref => Some("ref"),
                                hir::BindingAnnotation::RefMut => Some("ref mut"),
                            };
                            let ident = if let Some(binding) = binding {
                                format!("{} {}", binding, ident)
                            } else {
                                ident.to_string()
                            };
                            err.span_suggestion(
                                fieldpat.span,
                                "use shorthand field pattern",
                                ident,
                                Applicability::MachineApplicable,
                            );
                            err.emit();
                        });
                    }
                }
            }
        }
    }
}

declare_lint! {
    /// The `unsafe_code` lint catches usage of `unsafe` code.
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// #![deny(unsafe_code)]
    /// fn main() {
    ///     unsafe {
    ///
    ///     }
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// This lint is intended to restrict the usage of `unsafe`, which can be
    /// difficult to use correctly.
    UNSAFE_CODE,
    Allow,
    "usage of `unsafe` code"
}

declare_lint_pass!(UnsafeCode => [UNSAFE_CODE]);

impl UnsafeCode {
    fn report_unsafe(
        &self,
        cx: &EarlyContext<'_>,
        span: Span,
        decorate: impl for<'a> FnOnce(LintDiagnosticBuilder<'a>),
    ) {
        // This comes from a macro that has `#[allow_internal_unsafe]`.
        if span.allows_unsafe() {
            return;
        }

        cx.struct_span_lint(UNSAFE_CODE, span, decorate);
    }

    fn report_overriden_symbol_name(&self, cx: &EarlyContext<'_>, span: Span, msg: &str) {
        self.report_unsafe(cx, span, |lint| {
            lint.build(msg)
                .note(
                    "the linker's behavior with multiple libraries exporting duplicate symbol \
                    names is undefined and Rust cannot provide guarantees when you manually \
                    override them",
                )
                .emit();
        })
    }
}

impl EarlyLintPass for UnsafeCode {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &ast::Attribute) {
        if attr.has_name(sym::allow_internal_unsafe) {
            self.report_unsafe(cx, attr.span, |lint| {
                lint.build(
                    "`allow_internal_unsafe` allows defining \
                                               macros using unsafe without triggering \
                                               the `unsafe_code` lint at their call site",
                )
                .emit()
            });
        }
    }

    fn check_expr(&mut self, cx: &EarlyContext<'_>, e: &ast::Expr) {
        if let ast::ExprKind::Block(ref blk, _) = e.kind {
            // Don't warn about generated blocks; that'll just pollute the output.
            if blk.rules == ast::BlockCheckMode::Unsafe(ast::UserProvided) {
                self.report_unsafe(cx, blk.span, |lint| {
                    lint.build("usage of an `unsafe` block").emit()
                });
            }
        }
    }

    fn check_item(&mut self, cx: &EarlyContext<'_>, it: &ast::Item) {
        match it.kind {
            ast::ItemKind::Trait(box ast::Trait { unsafety: ast::Unsafe::Yes(_), .. }) => self
                .report_unsafe(cx, it.span, |lint| {
                    lint.build("declaration of an `unsafe` trait").emit()
                }),

            ast::ItemKind::Impl(box ast::Impl { unsafety: ast::Unsafe::Yes(_), .. }) => self
                .report_unsafe(cx, it.span, |lint| {
                    lint.build("implementation of an `unsafe` trait").emit()
                }),

            ast::ItemKind::Fn(..) => {
                if let Some(attr) = cx.sess().find_by_name(&it.attrs, sym::no_mangle) {
                    self.report_overriden_symbol_name(
                        cx,
                        attr.span,
                        "declaration of a `no_mangle` function",
                    );
                }
                if let Some(attr) = cx.sess().find_by_name(&it.attrs, sym::export_name) {
                    self.report_overriden_symbol_name(
                        cx,
                        attr.span,
                        "declaration of a function with `export_name`",
                    );
                }
            }

            ast::ItemKind::Static(..) => {
                if let Some(attr) = cx.sess().find_by_name(&it.attrs, sym::no_mangle) {
                    self.report_overriden_symbol_name(
                        cx,
                        attr.span,
                        "declaration of a `no_mangle` static",
                    );
                }
                if let Some(attr) = cx.sess().find_by_name(&it.attrs, sym::export_name) {
                    self.report_overriden_symbol_name(
                        cx,
                        attr.span,
                        "declaration of a static with `export_name`",
                    );
                }
            }

            _ => {}
        }
    }

    fn check_impl_item(&mut self, cx: &EarlyContext<'_>, it: &ast::AssocItem) {
        if let ast::AssocItemKind::Fn(..) = it.kind {
            if let Some(attr) = cx.sess().find_by_name(&it.attrs, sym::no_mangle) {
                self.report_overriden_symbol_name(
                    cx,
                    attr.span,
                    "declaration of a `no_mangle` method",
                );
            }
            if let Some(attr) = cx.sess().find_by_name(&it.attrs, sym::export_name) {
                self.report_overriden_symbol_name(
                    cx,
                    attr.span,
                    "declaration of a method with `export_name`",
                );
            }
        }
    }

    fn check_fn(&mut self, cx: &EarlyContext<'_>, fk: FnKind<'_>, span: Span, _: ast::NodeId) {
        if let FnKind::Fn(
            ctxt,
            _,
            ast::FnSig { header: ast::FnHeader { unsafety: ast::Unsafe::Yes(_), .. }, .. },
            _,
            body,
        ) = fk
        {
            let msg = match ctxt {
                FnCtxt::Foreign => return,
                FnCtxt::Free => "declaration of an `unsafe` function",
                FnCtxt::Assoc(_) if body.is_none() => "declaration of an `unsafe` method",
                FnCtxt::Assoc(_) => "implementation of an `unsafe` method",
            };
            self.report_unsafe(cx, span, |lint| lint.build(msg).emit());
        }
    }
}

declare_lint! {
    /// The `missing_docs` lint detects missing documentation for public items.
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// #![deny(missing_docs)]
    /// pub fn foo() {}
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// This lint is intended to ensure that a library is well-documented.
    /// Items without documentation can be difficult for users to understand
    /// how to use properly.
    ///
    /// This lint is "allow" by default because it can be noisy, and not all
    /// projects may want to enforce everything to be documented.
    pub MISSING_DOCS,
    Allow,
    "detects missing documentation for public members",
    report_in_external_macro
}

pub struct MissingDoc {
    /// Stack of whether `#[doc(hidden)]` is set at each level which has lint attributes.
    doc_hidden_stack: Vec<bool>,

    /// Private traits or trait items that leaked through. Don't check their methods.
    private_traits: FxHashSet<hir::HirId>,
}

impl_lint_pass!(MissingDoc => [MISSING_DOCS]);

fn has_doc(attr: &ast::Attribute) -> bool {
    if attr.is_doc_comment() {
        return true;
    }

    if !attr.has_name(sym::doc) {
        return false;
    }

    if attr.value_str().is_some() {
        return true;
    }

    if let Some(list) = attr.meta_item_list() {
        for meta in list {
            if meta.has_name(sym::hidden) {
                return true;
            }
        }
    }

    false
}

impl MissingDoc {
    pub fn new() -> MissingDoc {
        MissingDoc { doc_hidden_stack: vec![false], private_traits: FxHashSet::default() }
    }

    fn doc_hidden(&self) -> bool {
        *self.doc_hidden_stack.last().expect("empty doc_hidden_stack")
    }

    fn check_missing_docs_attrs(
        &self,
        cx: &LateContext<'_>,
        def_id: LocalDefId,
        sp: Span,
        article: &'static str,
        desc: &'static str,
    ) {
        // If we're building a test harness, then warning about
        // documentation is probably not really relevant right now.
        if cx.sess().opts.test {
            return;
        }

        // `#[doc(hidden)]` disables missing_docs check.
        if self.doc_hidden() {
            return;
        }

        // Only check publicly-visible items, using the result from the privacy pass.
        // It's an option so the crate root can also use this function (it doesn't
        // have a `NodeId`).
        if def_id != CRATE_DEF_ID {
            if !cx.access_levels.is_exported(def_id) {
                return;
            }
        }

        let attrs = cx.tcx.get_attrs(def_id.to_def_id());
        let has_doc = attrs.iter().any(has_doc);
        if !has_doc {
            cx.struct_span_lint(
                MISSING_DOCS,
                cx.tcx.sess.source_map().guess_head_span(sp),
                |lint| {
                    lint.build(&format!("missing documentation for {} {}", article, desc)).emit()
                },
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for MissingDoc {
    fn enter_lint_attrs(&mut self, _cx: &LateContext<'_>, attrs: &[ast::Attribute]) {
        let doc_hidden = self.doc_hidden()
            || attrs.iter().any(|attr| {
                attr.has_name(sym::doc)
                    && match attr.meta_item_list() {
                        None => false,
                        Some(l) => attr::list_contains_name(&l, sym::hidden),
                    }
            });
        self.doc_hidden_stack.push(doc_hidden);
    }

    fn exit_lint_attrs(&mut self, _: &LateContext<'_>, _attrs: &[ast::Attribute]) {
        self.doc_hidden_stack.pop().expect("empty doc_hidden_stack");
    }

    fn check_crate(&mut self, cx: &LateContext<'_>) {
        self.check_missing_docs_attrs(
            cx,
            CRATE_DEF_ID,
            cx.tcx.def_span(CRATE_DEF_ID),
            "the",
            "crate",
        );
    }

    fn check_item(&mut self, cx: &LateContext<'_>, it: &hir::Item<'_>) {
        match it.kind {
            hir::ItemKind::Trait(.., trait_item_refs) => {
                // Issue #11592: traits are always considered exported, even when private.
                if let hir::VisibilityKind::Inherited = it.vis.node {
                    self.private_traits.insert(it.hir_id());
                    for trait_item_ref in trait_item_refs {
                        self.private_traits.insert(trait_item_ref.id.hir_id());
                    }
                    return;
                }
            }
            hir::ItemKind::Impl(hir::Impl { of_trait: Some(ref trait_ref), items, .. }) => {
                // If the trait is private, add the impl items to `private_traits` so they don't get
                // reported for missing docs.
                let real_trait = trait_ref.path.res.def_id();
                let Some(def_id) = real_trait.as_local() else { return };
                let hir_id = cx.tcx.hir().local_def_id_to_hir_id(def_id);
                let Some(Node::Item(item)) = cx.tcx.hir().find(hir_id) else { return };
                if let hir::VisibilityKind::Inherited = item.vis.node {
                    for impl_item_ref in items {
                        self.private_traits.insert(impl_item_ref.id.hir_id());
                    }
                }
                return;
            }

            hir::ItemKind::TyAlias(..)
            | hir::ItemKind::Fn(..)
            | hir::ItemKind::Macro(..)
            | hir::ItemKind::Mod(..)
            | hir::ItemKind::Enum(..)
            | hir::ItemKind::Struct(..)
            | hir::ItemKind::Union(..)
            | hir::ItemKind::Const(..)
            | hir::ItemKind::Static(..) => {}

            _ => return,
        };

        let (article, desc) = cx.tcx.article_and_description(it.def_id.to_def_id());

        self.check_missing_docs_attrs(cx, it.def_id, it.span, article, desc);
    }

    fn check_trait_item(&mut self, cx: &LateContext<'_>, trait_item: &hir::TraitItem<'_>) {
        if self.private_traits.contains(&trait_item.hir_id()) {
            return;
        }

        let (article, desc) = cx.tcx.article_and_description(trait_item.def_id.to_def_id());

        self.check_missing_docs_attrs(cx, trait_item.def_id, trait_item.span, article, desc);
    }

    fn check_impl_item(&mut self, cx: &LateContext<'_>, impl_item: &hir::ImplItem<'_>) {
        // If the method is an impl for a trait, don't doc.
        if method_context(cx, impl_item.hir_id()) == MethodLateContext::TraitImpl {
            return;
        }

        // If the method is an impl for an item with docs_hidden, don't doc.
        if method_context(cx, impl_item.hir_id()) == MethodLateContext::PlainImpl {
            let parent = cx.tcx.hir().get_parent_did(impl_item.hir_id());
            let impl_ty = cx.tcx.type_of(parent);
            let outerdef = match impl_ty.kind() {
                ty::Adt(def, _) => Some(def.did),
                ty::Foreign(def_id) => Some(*def_id),
                _ => None,
            };
            let is_hidden = match outerdef {
                Some(id) => cx.tcx.is_doc_hidden(id),
                None => false,
            };
            if is_hidden {
                return;
            }
        }

        let (article, desc) = cx.tcx.article_and_description(impl_item.def_id.to_def_id());
        self.check_missing_docs_attrs(cx, impl_item.def_id, impl_item.span, article, desc);
    }

    fn check_foreign_item(&mut self, cx: &LateContext<'_>, foreign_item: &hir::ForeignItem<'_>) {
        let (article, desc) = cx.tcx.article_and_description(foreign_item.def_id.to_def_id());
        self.check_missing_docs_attrs(cx, foreign_item.def_id, foreign_item.span, article, desc);
    }

    fn check_field_def(&mut self, cx: &LateContext<'_>, sf: &hir::FieldDef<'_>) {
        if !sf.is_positional() {
            let def_id = cx.tcx.hir().local_def_id(sf.hir_id);
            self.check_missing_docs_attrs(cx, def_id, sf.span, "a", "struct field")
        }
    }

    fn check_variant(&mut self, cx: &LateContext<'_>, v: &hir::Variant<'_>) {
        self.check_missing_docs_attrs(cx, cx.tcx.hir().local_def_id(v.id), v.span, "a", "variant");
    }
}

declare_lint! {
    /// The `missing_copy_implementations` lint detects potentially-forgotten
    /// implementations of [`Copy`].
    ///
    /// [`Copy`]: https://doc.rust-lang.org/std/marker/trait.Copy.html
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// #![deny(missing_copy_implementations)]
    /// pub struct Foo {
    ///     pub field: i32
    /// }
    /// # fn main() {}
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Historically (before 1.0), types were automatically marked as `Copy`
    /// if possible. This was changed so that it required an explicit opt-in
    /// by implementing the `Copy` trait. As part of this change, a lint was
    /// added to alert if a copyable type was not marked `Copy`.
    ///
    /// This lint is "allow" by default because this code isn't bad; it is
    /// common to write newtypes like this specifically so that a `Copy` type
    /// is no longer `Copy`. `Copy` types can result in unintended copies of
    /// large data which can impact performance.
    pub MISSING_COPY_IMPLEMENTATIONS,
    Allow,
    "detects potentially-forgotten implementations of `Copy`"
}

declare_lint_pass!(MissingCopyImplementations => [MISSING_COPY_IMPLEMENTATIONS]);

impl<'tcx> LateLintPass<'tcx> for MissingCopyImplementations {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &hir::Item<'_>) {
        if !cx.access_levels.is_reachable(item.def_id) {
            return;
        }
        let (def, ty) = match item.kind {
            hir::ItemKind::Struct(_, ref ast_generics) => {
                if !ast_generics.params.is_empty() {
                    return;
                }
                let def = cx.tcx.adt_def(item.def_id);
                (def, cx.tcx.mk_adt(def, cx.tcx.intern_substs(&[])))
            }
            hir::ItemKind::Union(_, ref ast_generics) => {
                if !ast_generics.params.is_empty() {
                    return;
                }
                let def = cx.tcx.adt_def(item.def_id);
                (def, cx.tcx.mk_adt(def, cx.tcx.intern_substs(&[])))
            }
            hir::ItemKind::Enum(_, ref ast_generics) => {
                if !ast_generics.params.is_empty() {
                    return;
                }
                let def = cx.tcx.adt_def(item.def_id);
                (def, cx.tcx.mk_adt(def, cx.tcx.intern_substs(&[])))
            }
            _ => return,
        };
        if def.has_dtor(cx.tcx) {
            return;
        }
        let param_env = ty::ParamEnv::empty();
        if ty.is_copy_modulo_regions(cx.tcx.at(item.span), param_env) {
            return;
        }
        if can_type_implement_copy(cx.tcx, param_env, ty).is_ok() {
            cx.struct_span_lint(MISSING_COPY_IMPLEMENTATIONS, item.span, |lint| {
                lint.build(
                    "type could implement `Copy`; consider adding `impl \
                          Copy`",
                )
                .emit()
            })
        }
    }
}

declare_lint! {
    /// The `missing_debug_implementations` lint detects missing
    /// implementations of [`fmt::Debug`].
    ///
    /// [`fmt::Debug`]: https://doc.rust-lang.org/std/fmt/trait.Debug.html
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// #![deny(missing_debug_implementations)]
    /// pub struct Foo;
    /// # fn main() {}
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Having a `Debug` implementation on all types can assist with
    /// debugging, as it provides a convenient way to format and display a
    /// value. Using the `#[derive(Debug)]` attribute will automatically
    /// generate a typical implementation, or a custom implementation can be
    /// added by manually implementing the `Debug` trait.
    ///
    /// This lint is "allow" by default because adding `Debug` to all types can
    /// have a negative impact on compile time and code size. It also requires
    /// boilerplate to be added to every type, which can be an impediment.
    MISSING_DEBUG_IMPLEMENTATIONS,
    Allow,
    "detects missing implementations of Debug"
}

#[derive(Default)]
pub struct MissingDebugImplementations {
    impling_types: Option<LocalDefIdSet>,
}

impl_lint_pass!(MissingDebugImplementations => [MISSING_DEBUG_IMPLEMENTATIONS]);

impl<'tcx> LateLintPass<'tcx> for MissingDebugImplementations {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &hir::Item<'_>) {
        if !cx.access_levels.is_reachable(item.def_id) {
            return;
        }

        match item.kind {
            hir::ItemKind::Struct(..) | hir::ItemKind::Union(..) | hir::ItemKind::Enum(..) => {}
            _ => return,
        }

        let Some(debug) = cx.tcx.get_diagnostic_item(sym::Debug) else {
            return
        };

        if self.impling_types.is_none() {
            let mut impls = LocalDefIdSet::default();
            cx.tcx.for_each_impl(debug, |d| {
                if let Some(ty_def) = cx.tcx.type_of(d).ty_adt_def() {
                    if let Some(def_id) = ty_def.did.as_local() {
                        impls.insert(def_id);
                    }
                }
            });

            self.impling_types = Some(impls);
            debug!("{:?}", self.impling_types);
        }

        if !self.impling_types.as_ref().unwrap().contains(&item.def_id) {
            cx.struct_span_lint(MISSING_DEBUG_IMPLEMENTATIONS, item.span, |lint| {
                lint.build(&format!(
                    "type does not implement `{}`; consider adding `#[derive(Debug)]` \
                     or a manual implementation",
                    cx.tcx.def_path_str(debug)
                ))
                .emit()
            });
        }
    }
}

declare_lint! {
    /// The `anonymous_parameters` lint detects anonymous parameters in trait
    /// definitions.
    ///
    /// ### Example
    ///
    /// ```rust,edition2015,compile_fail
    /// #![deny(anonymous_parameters)]
    /// // edition 2015
    /// pub trait Foo {
    ///     fn foo(usize);
    /// }
    /// fn main() {}
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// This syntax is mostly a historical accident, and can be worked around
    /// quite easily by adding an `_` pattern or a descriptive identifier:
    ///
    /// ```rust
    /// trait Foo {
    ///     fn foo(_: usize);
    /// }
    /// ```
    ///
    /// This syntax is now a hard error in the 2018 edition. In the 2015
    /// edition, this lint is "warn" by default. This lint
    /// enables the [`cargo fix`] tool with the `--edition` flag to
    /// automatically transition old code from the 2015 edition to 2018. The
    /// tool will run this lint and automatically apply the
    /// suggested fix from the compiler (which is to add `_` to each
    /// parameter). This provides a completely automated way to update old
    /// code for a new edition. See [issue #41686] for more details.
    ///
    /// [issue #41686]: https://github.com/rust-lang/rust/issues/41686
    /// [`cargo fix`]: https://doc.rust-lang.org/cargo/commands/cargo-fix.html
    pub ANONYMOUS_PARAMETERS,
    Warn,
    "detects anonymous parameters",
    @future_incompatible = FutureIncompatibleInfo {
        reference: "issue #41686 <https://github.com/rust-lang/rust/issues/41686>",
        reason: FutureIncompatibilityReason::EditionError(Edition::Edition2018),
    };
}

declare_lint_pass!(
    /// Checks for use of anonymous parameters (RFC 1685).
    AnonymousParameters => [ANONYMOUS_PARAMETERS]
);

impl EarlyLintPass for AnonymousParameters {
    fn check_trait_item(&mut self, cx: &EarlyContext<'_>, it: &ast::AssocItem) {
        if cx.sess.edition() != Edition::Edition2015 {
            // This is a hard error in future editions; avoid linting and erroring
            return;
        }
        if let ast::AssocItemKind::Fn(box Fn { ref sig, .. }) = it.kind {
            for arg in sig.decl.inputs.iter() {
                if let ast::PatKind::Ident(_, ident, None) = arg.pat.kind {
                    if ident.name == kw::Empty {
                        cx.struct_span_lint(ANONYMOUS_PARAMETERS, arg.pat.span, |lint| {
                            let ty_snip = cx.sess.source_map().span_to_snippet(arg.ty.span);

                            let (ty_snip, appl) = if let Ok(ref snip) = ty_snip {
                                (snip.as_str(), Applicability::MachineApplicable)
                            } else {
                                ("<type>", Applicability::HasPlaceholders)
                            };

                            lint.build(
                                "anonymous parameters are deprecated and will be \
                                     removed in the next edition",
                            )
                            .span_suggestion(
                                arg.pat.span,
                                "try naming the parameter or explicitly \
                                            ignoring it",
                                format!("_: {}", ty_snip),
                                appl,
                            )
                            .emit();
                        })
                    }
                }
            }
        }
    }
}

/// Check for use of attributes which have been deprecated.
#[derive(Clone)]
pub struct DeprecatedAttr {
    // This is not free to compute, so we want to keep it around, rather than
    // compute it for every attribute.
    depr_attrs: Vec<&'static BuiltinAttribute>,
}

impl_lint_pass!(DeprecatedAttr => []);

impl DeprecatedAttr {
    pub fn new() -> DeprecatedAttr {
        DeprecatedAttr { depr_attrs: deprecated_attributes() }
    }
}

fn lint_deprecated_attr(
    cx: &EarlyContext<'_>,
    attr: &ast::Attribute,
    msg: &str,
    suggestion: Option<&str>,
) {
    cx.struct_span_lint(DEPRECATED, attr.span, |lint| {
        lint.build(msg)
            .span_suggestion_short(
                attr.span,
                suggestion.unwrap_or("remove this attribute"),
                String::new(),
                Applicability::MachineApplicable,
            )
            .emit();
    })
}

impl EarlyLintPass for DeprecatedAttr {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &ast::Attribute) {
        for BuiltinAttribute { name, gate, .. } in &self.depr_attrs {
            if attr.ident().map(|ident| ident.name) == Some(*name) {
                if let &AttributeGate::Gated(
                    Stability::Deprecated(link, suggestion),
                    name,
                    reason,
                    _,
                ) = gate
                {
                    let msg =
                        format!("use of deprecated attribute `{}`: {}. See {}", name, reason, link);
                    lint_deprecated_attr(cx, attr, &msg, suggestion);
                }
                return;
            }
        }
        if attr.has_name(sym::no_start) || attr.has_name(sym::crate_id) {
            let path_str = pprust::path_to_string(&attr.get_normal_item().path);
            let msg = format!("use of deprecated attribute `{}`: no longer used.", path_str);
            lint_deprecated_attr(cx, attr, &msg, None);
        }
    }
}

fn warn_if_doc(cx: &EarlyContext<'_>, node_span: Span, node_kind: &str, attrs: &[ast::Attribute]) {
    use rustc_ast::token::CommentKind;

    let mut attrs = attrs.iter().peekable();

    // Accumulate a single span for sugared doc comments.
    let mut sugared_span: Option<Span> = None;

    while let Some(attr) = attrs.next() {
        let is_doc_comment = attr.is_doc_comment();
        if is_doc_comment {
            sugared_span =
                Some(sugared_span.map_or(attr.span, |span| span.with_hi(attr.span.hi())));
        }

        if attrs.peek().map_or(false, |next_attr| next_attr.is_doc_comment()) {
            continue;
        }

        let span = sugared_span.take().unwrap_or(attr.span);

        if is_doc_comment || attr.has_name(sym::doc) {
            cx.struct_span_lint(UNUSED_DOC_COMMENTS, span, |lint| {
                let mut err = lint.build("unused doc comment");
                err.span_label(
                    node_span,
                    format!("rustdoc does not generate documentation for {}", node_kind),
                );
                match attr.kind {
                    AttrKind::DocComment(CommentKind::Line, _) | AttrKind::Normal(..) => {
                        err.help("use `//` for a plain comment");
                    }
                    AttrKind::DocComment(CommentKind::Block, _) => {
                        err.help("use `/* */` for a plain comment");
                    }
                }
                err.emit();
            });
        }
    }
}

impl EarlyLintPass for UnusedDocComment {
    fn check_stmt(&mut self, cx: &EarlyContext<'_>, stmt: &ast::Stmt) {
        let kind = match stmt.kind {
            ast::StmtKind::Local(..) => "statements",
            // Disabled pending discussion in #78306
            ast::StmtKind::Item(..) => return,
            // expressions will be reported by `check_expr`.
            ast::StmtKind::Empty
            | ast::StmtKind::Semi(_)
            | ast::StmtKind::Expr(_)
            | ast::StmtKind::MacCall(_) => return,
        };

        warn_if_doc(cx, stmt.span, kind, stmt.kind.attrs());
    }

    fn check_arm(&mut self, cx: &EarlyContext<'_>, arm: &ast::Arm) {
        let arm_span = arm.pat.span.with_hi(arm.body.span.hi());
        warn_if_doc(cx, arm_span, "match arms", &arm.attrs);
    }

    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &ast::Expr) {
        warn_if_doc(cx, expr.span, "expressions", &expr.attrs);
    }

    fn check_generic_param(&mut self, cx: &EarlyContext<'_>, param: &ast::GenericParam) {
        warn_if_doc(cx, param.ident.span, "generic parameters", &param.attrs);
    }
}

declare_lint! {
    /// The `no_mangle_const_items` lint detects any `const` items with the
    /// [`no_mangle` attribute].
    ///
    /// [`no_mangle` attribute]: https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// #[no_mangle]
    /// const FOO: i32 = 5;
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Constants do not have their symbols exported, and therefore, this
    /// probably means you meant to use a [`static`], not a [`const`].
    ///
    /// [`static`]: https://doc.rust-lang.org/reference/items/static-items.html
    /// [`const`]: https://doc.rust-lang.org/reference/items/constant-items.html
    NO_MANGLE_CONST_ITEMS,
    Deny,
    "const items will not have their symbols exported"
}

declare_lint! {
    /// The `no_mangle_generic_items` lint detects generic items that must be
    /// mangled.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #[no_mangle]
    /// fn foo<T>(t: T) {
    ///
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// A function with generics must have its symbol mangled to accommodate
    /// the generic parameter. The [`no_mangle` attribute] has no effect in
    /// this situation, and should be removed.
    ///
    /// [`no_mangle` attribute]: https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute
    NO_MANGLE_GENERIC_ITEMS,
    Warn,
    "generic items must be mangled"
}

declare_lint_pass!(InvalidNoMangleItems => [NO_MANGLE_CONST_ITEMS, NO_MANGLE_GENERIC_ITEMS]);

impl<'tcx> LateLintPass<'tcx> for InvalidNoMangleItems {
    fn check_item(&mut self, cx: &LateContext<'_>, it: &hir::Item<'_>) {
        let attrs = cx.tcx.hir().attrs(it.hir_id());
        let check_no_mangle_on_generic_fn = |no_mangle_attr: &ast::Attribute,
                                             impl_generics: Option<&hir::Generics<'_>>,
                                             generics: &hir::Generics<'_>,
                                             span| {
            for param in
                generics.params.iter().chain(impl_generics.map(|g| g.params).into_iter().flatten())
            {
                match param.kind {
                    GenericParamKind::Lifetime { .. } => {}
                    GenericParamKind::Type { .. } | GenericParamKind::Const { .. } => {
                        cx.struct_span_lint(NO_MANGLE_GENERIC_ITEMS, span, |lint| {
                            lint.build("functions generic over types or consts must be mangled")
                                .span_suggestion_short(
                                    no_mangle_attr.span,
                                    "remove this attribute",
                                    String::new(),
                                    // Use of `#[no_mangle]` suggests FFI intent; correct
                                    // fix may be to monomorphize source by hand
                                    Applicability::MaybeIncorrect,
                                )
                                .emit();
                        });
                        break;
                    }
                }
            }
        };
        match it.kind {
            hir::ItemKind::Fn(.., ref generics, _) => {
                if let Some(no_mangle_attr) = cx.sess().find_by_name(attrs, sym::no_mangle) {
                    check_no_mangle_on_generic_fn(no_mangle_attr, None, generics, it.span);
                }
            }
            hir::ItemKind::Const(..) => {
                if cx.sess().contains_name(attrs, sym::no_mangle) {
                    // Const items do not refer to a particular location in memory, and therefore
                    // don't have anything to attach a symbol to
                    cx.struct_span_lint(NO_MANGLE_CONST_ITEMS, it.span, |lint| {
                        let msg = "const items should never be `#[no_mangle]`";
                        let mut err = lint.build(msg);

                        // account for "pub const" (#45562)
                        let start = cx
                            .tcx
                            .sess
                            .source_map()
                            .span_to_snippet(it.span)
                            .map(|snippet| snippet.find("const").unwrap_or(0))
                            .unwrap_or(0) as u32;
                        // `const` is 5 chars
                        let const_span = it.span.with_hi(BytePos(it.span.lo().0 + start + 5));
                        err.span_suggestion(
                            const_span,
                            "try a static value",
                            "pub static".to_owned(),
                            Applicability::MachineApplicable,
                        );
                        err.emit();
                    });
                }
            }
            hir::ItemKind::Impl(hir::Impl { ref generics, items, .. }) => {
                for it in items {
                    if let hir::AssocItemKind::Fn { .. } = it.kind {
                        if let Some(no_mangle_attr) = cx
                            .sess()
                            .find_by_name(cx.tcx.hir().attrs(it.id.hir_id()), sym::no_mangle)
                        {
                            check_no_mangle_on_generic_fn(
                                no_mangle_attr,
                                Some(generics),
                                cx.tcx.hir().get_generics(it.id.def_id.to_def_id()).unwrap(),
                                it.span,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

declare_lint! {
    /// The `mutable_transmutes` lint catches transmuting from `&T` to `&mut
    /// T` because it is [undefined behavior].
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// unsafe {
    ///     let y = std::mem::transmute::<&i32, &mut i32>(&5);
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Certain assumptions are made about aliasing of data, and this transmute
    /// violates those assumptions. Consider using [`UnsafeCell`] instead.
    ///
    /// [`UnsafeCell`]: https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html
    MUTABLE_TRANSMUTES,
    Deny,
    "mutating transmuted &mut T from &T may cause undefined behavior"
}

declare_lint_pass!(MutableTransmutes => [MUTABLE_TRANSMUTES]);

impl<'tcx> LateLintPass<'tcx> for MutableTransmutes {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &hir::Expr<'_>) {
        use rustc_target::spec::abi::Abi::RustIntrinsic;
        if let Some((&ty::Ref(_, _, from_mt), &ty::Ref(_, _, to_mt))) =
            get_transmute_from_to(cx, expr).map(|(ty1, ty2)| (ty1.kind(), ty2.kind()))
        {
            if to_mt == hir::Mutability::Mut && from_mt == hir::Mutability::Not {
                let msg = "mutating transmuted &mut T from &T may cause undefined behavior, \
                               consider instead using an UnsafeCell";
                cx.struct_span_lint(MUTABLE_TRANSMUTES, expr.span, |lint| lint.build(msg).emit());
            }
        }

        fn get_transmute_from_to<'tcx>(
            cx: &LateContext<'tcx>,
            expr: &hir::Expr<'_>,
        ) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
            let def = if let hir::ExprKind::Path(ref qpath) = expr.kind {
                cx.qpath_res(qpath, expr.hir_id)
            } else {
                return None;
            };
            if let Res::Def(DefKind::Fn, did) = def {
                if !def_id_is_transmute(cx, did) {
                    return None;
                }
                let sig = cx.typeck_results().node_type(expr.hir_id).fn_sig(cx.tcx);
                let from = sig.inputs().skip_binder()[0];
                let to = sig.output().skip_binder();
                return Some((from, to));
            }
            None
        }

        fn def_id_is_transmute(cx: &LateContext<'_>, def_id: DefId) -> bool {
            cx.tcx.fn_sig(def_id).abi() == RustIntrinsic
                && cx.tcx.item_name(def_id) == sym::transmute
        }
    }
}

declare_lint! {
    /// The `unstable_features` is deprecated and should no longer be used.
    UNSTABLE_FEATURES,
    Allow,
    "enabling unstable features (deprecated. do not use)"
}

declare_lint_pass!(
    /// Forbids using the `#[feature(...)]` attribute
    UnstableFeatures => [UNSTABLE_FEATURES]
);

impl<'tcx> LateLintPass<'tcx> for UnstableFeatures {
    fn check_attribute(&mut self, cx: &LateContext<'_>, attr: &ast::Attribute) {
        if attr.has_name(sym::feature) {
            if let Some(items) = attr.meta_item_list() {
                for item in items {
                    cx.struct_span_lint(UNSTABLE_FEATURES, item.span(), |lint| {
                        lint.build("unstable feature").emit()
                    });
                }
            }
        }
    }
}

declare_lint! {
    /// The `unreachable_pub` lint triggers for `pub` items not reachable from
    /// the crate root.
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// #![deny(unreachable_pub)]
    /// mod foo {
    ///     pub mod bar {
    ///
    ///     }
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// A bare `pub` visibility may be misleading if the item is not actually
    /// publicly exported from the crate. The `pub(crate)` visibility is
    /// recommended to be used instead, which more clearly expresses the intent
    /// that the item is only visible within its own crate.
    ///
    /// This lint is "allow" by default because it will trigger for a large
    /// amount existing Rust code, and has some false-positives. Eventually it
    /// is desired for this to become warn-by-default.
    pub UNREACHABLE_PUB,
    Allow,
    "`pub` items not reachable from crate root"
}

declare_lint_pass!(
    /// Lint for items marked `pub` that aren't reachable from other crates.
    UnreachablePub => [UNREACHABLE_PUB]
);

impl UnreachablePub {
    fn perform_lint(
        &self,
        cx: &LateContext<'_>,
        what: &str,
        def_id: LocalDefId,
        vis: &hir::Visibility<'_>,
        span: Span,
        exportable: bool,
    ) {
        let mut applicability = Applicability::MachineApplicable;
        match vis.node {
            hir::VisibilityKind::Public if !cx.access_levels.is_reachable(def_id) => {
                if span.from_expansion() {
                    applicability = Applicability::MaybeIncorrect;
                }
                let def_span = cx.tcx.sess.source_map().guess_head_span(span);
                cx.struct_span_lint(UNREACHABLE_PUB, def_span, |lint| {
                    let mut err = lint.build(&format!("unreachable `pub` {}", what));
                    let replacement = if cx.tcx.features().crate_visibility_modifier {
                        "crate"
                    } else {
                        "pub(crate)"
                    }
                    .to_owned();

                    err.span_suggestion(
                        vis.span,
                        "consider restricting its visibility",
                        replacement,
                        applicability,
                    );
                    if exportable {
                        err.help("or consider exporting it for use by other crates");
                    }
                    err.emit();
                });
            }
            _ => {}
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnreachablePub {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &hir::Item<'_>) {
        self.perform_lint(cx, "item", item.def_id, &item.vis, item.span, true);
    }

    fn check_foreign_item(&mut self, cx: &LateContext<'_>, foreign_item: &hir::ForeignItem<'tcx>) {
        self.perform_lint(
            cx,
            "item",
            foreign_item.def_id,
            &foreign_item.vis,
            foreign_item.span,
            true,
        );
    }

    fn check_field_def(&mut self, cx: &LateContext<'_>, field: &hir::FieldDef<'_>) {
        let def_id = cx.tcx.hir().local_def_id(field.hir_id);
        self.perform_lint(cx, "field", def_id, &field.vis, field.span, false);
    }

    fn check_impl_item(&mut self, cx: &LateContext<'_>, impl_item: &hir::ImplItem<'_>) {
        self.perform_lint(cx, "item", impl_item.def_id, &impl_item.vis, impl_item.span, false);
    }
}

declare_lint! {
    /// The `type_alias_bounds` lint detects bounds in type aliases.
    ///
    /// ### Example
    ///
    /// ```rust
    /// type SendVec<T: Send> = Vec<T>;
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// The trait bounds in a type alias are currently ignored, and should not
    /// be included to avoid confusion. This was previously allowed
    /// unintentionally; this may become a hard error in the future.
    TYPE_ALIAS_BOUNDS,
    Warn,
    "bounds in type aliases are not enforced"
}

declare_lint_pass!(
    /// Lint for trait and lifetime bounds in type aliases being mostly ignored.
    /// They are relevant when using associated types, but otherwise neither checked
    /// at definition site nor enforced at use site.
    TypeAliasBounds => [TYPE_ALIAS_BOUNDS]
);

impl TypeAliasBounds {
    fn is_type_variable_assoc(qpath: &hir::QPath<'_>) -> bool {
        match *qpath {
            hir::QPath::TypeRelative(ref ty, _) => {
                // If this is a type variable, we found a `T::Assoc`.
                match ty.kind {
                    hir::TyKind::Path(hir::QPath::Resolved(None, ref path)) => {
                        matches!(path.res, Res::Def(DefKind::TyParam, _))
                    }
                    _ => false,
                }
            }
            hir::QPath::Resolved(..) | hir::QPath::LangItem(..) => false,
        }
    }

    fn suggest_changing_assoc_types(ty: &hir::Ty<'_>, err: &mut DiagnosticBuilder<'_>) {
        // Access to associates types should use `<T as Bound>::Assoc`, which does not need a
        // bound.  Let's see if this type does that.

        // We use a HIR visitor to walk the type.
        use rustc_hir::intravisit::{self, Visitor};
        struct WalkAssocTypes<'a, 'db> {
            err: &'a mut DiagnosticBuilder<'db>,
        }
        impl<'a, 'db, 'v> Visitor<'v> for WalkAssocTypes<'a, 'db> {
            type Map = intravisit::ErasedMap<'v>;

            fn nested_visit_map(&mut self) -> intravisit::NestedVisitorMap<Self::Map> {
                intravisit::NestedVisitorMap::None
            }

            fn visit_qpath(&mut self, qpath: &'v hir::QPath<'v>, id: hir::HirId, span: Span) {
                if TypeAliasBounds::is_type_variable_assoc(qpath) {
                    self.err.span_help(
                        span,
                        "use fully disambiguated paths (i.e., `<T as Trait>::Assoc`) to refer to \
                         associated types in type aliases",
                    );
                }
                intravisit::walk_qpath(self, qpath, id, span)
            }
        }

        // Let's go for a walk!
        let mut visitor = WalkAssocTypes { err };
        visitor.visit_ty(ty);
    }
}

impl<'tcx> LateLintPass<'tcx> for TypeAliasBounds {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &hir::Item<'_>) {
        let hir::ItemKind::TyAlias(ty, type_alias_generics) = &item.kind else {
            return
        };
        if let hir::TyKind::OpaqueDef(..) = ty.kind {
            // Bounds are respected for `type X = impl Trait`
            return;
        }
        let mut suggested_changing_assoc_types = false;
        // There must not be a where clause
        if !type_alias_generics.where_clause.predicates.is_empty() {
            cx.lint(
                TYPE_ALIAS_BOUNDS,
                |lint| {
                    let mut err = lint.build("where clauses are not enforced in type aliases");
                    let spans: Vec<_> = type_alias_generics
                        .where_clause
                        .predicates
                        .iter()
                        .map(|pred| pred.span())
                        .collect();
                    err.set_span(spans);
                    err.span_suggestion(
                        type_alias_generics.where_clause.span_for_predicates_or_empty_place(),
                        "the clause will not be checked when the type alias is used, and should be removed",
                        String::new(),
                        Applicability::MachineApplicable,
                    );
                    if !suggested_changing_assoc_types {
                        TypeAliasBounds::suggest_changing_assoc_types(ty, &mut err);
                        suggested_changing_assoc_types = true;
                    }
                    err.emit();
                },
            );
        }
        // The parameters must not have bounds
        for param in type_alias_generics.params.iter() {
            let spans: Vec<_> = param.bounds.iter().map(|b| b.span()).collect();
            let suggestion = spans
                .iter()
                .map(|sp| {
                    let start = param.span.between(*sp); // Include the `:` in `T: Bound`.
                    (start.to(*sp), String::new())
                })
                .collect();
            if !spans.is_empty() {
                cx.struct_span_lint(TYPE_ALIAS_BOUNDS, spans, |lint| {
                    let mut err =
                        lint.build("bounds on generic parameters are not enforced in type aliases");
                    let msg = "the bound will not be checked when the type alias is used, \
                                   and should be removed";
                    err.multipart_suggestion(&msg, suggestion, Applicability::MachineApplicable);
                    if !suggested_changing_assoc_types {
                        TypeAliasBounds::suggest_changing_assoc_types(ty, &mut err);
                        suggested_changing_assoc_types = true;
                    }
                    err.emit();
                });
            }
        }
    }
}

declare_lint_pass!(
    /// Lint constants that are erroneous.
    /// Without this lint, we might not get any diagnostic if the constant is
    /// unused within this crate, even though downstream crates can't use it
    /// without producing an error.
    UnusedBrokenConst => []
);

impl<'tcx> LateLintPass<'tcx> for UnusedBrokenConst {
    fn check_item(&mut self, cx: &LateContext<'_>, it: &hir::Item<'_>) {
        match it.kind {
            hir::ItemKind::Const(_, body_id) => {
                let def_id = cx.tcx.hir().body_owner_def_id(body_id).to_def_id();
                // trigger the query once for all constants since that will already report the errors
                // FIXME: Use ensure here
                let _ = cx.tcx.const_eval_poly(def_id);
            }
            hir::ItemKind::Static(_, _, body_id) => {
                let def_id = cx.tcx.hir().body_owner_def_id(body_id).to_def_id();
                // FIXME: Use ensure here
                let _ = cx.tcx.eval_static_initializer(def_id);
            }
            _ => {}
        }
    }
}

declare_lint! {
    /// The `trivial_bounds` lint detects trait bounds that don't depend on
    /// any type parameters.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #![feature(trivial_bounds)]
    /// pub struct A where i32: Copy;
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Usually you would not write a trait bound that you know is always
    /// true, or never true. However, when using macros, the macro may not
    /// know whether or not the constraint would hold or not at the time when
    /// generating the code. Currently, the compiler does not alert you if the
    /// constraint is always true, and generates an error if it is never true.
    /// The `trivial_bounds` feature changes this to be a warning in both
    /// cases, giving macros more freedom and flexibility to generate code,
    /// while still providing a signal when writing non-macro code that
    /// something is amiss.
    ///
    /// See [RFC 2056] for more details. This feature is currently only
    /// available on the nightly channel, see [tracking issue #48214].
    ///
    /// [RFC 2056]: https://github.com/rust-lang/rfcs/blob/master/text/2056-allow-trivial-where-clause-constraints.md
    /// [tracking issue #48214]: https://github.com/rust-lang/rust/issues/48214
    TRIVIAL_BOUNDS,
    Warn,
    "these bounds don't depend on an type parameters"
}

declare_lint_pass!(
    /// Lint for trait and lifetime bounds that don't depend on type parameters
    /// which either do nothing, or stop the item from being used.
    TrivialConstraints => [TRIVIAL_BOUNDS]
);

impl<'tcx> LateLintPass<'tcx> for TrivialConstraints {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        use rustc_middle::ty::fold::TypeFoldable;
        use rustc_middle::ty::PredicateKind::*;

        if cx.tcx.features().trivial_bounds {
            let predicates = cx.tcx.predicates_of(item.def_id);
            for &(predicate, span) in predicates.predicates {
                let predicate_kind_name = match predicate.kind().skip_binder() {
                    Trait(..) => "trait",
                    TypeOutlives(..) |
                    RegionOutlives(..) => "lifetime",

                    // Ignore projections, as they can only be global
                    // if the trait bound is global
                    Projection(..) |
                    // Ignore bounds that a user can't type
                    WellFormed(..) |
                    ObjectSafe(..) |
                    ClosureKind(..) |
                    Subtype(..) |
                    Coerce(..) |
                    ConstEvaluatable(..) |
                    ConstEquate(..) |
                    TypeWellFormedFromEnv(..) => continue,
                };
                if predicate.is_global(cx.tcx) {
                    cx.struct_span_lint(TRIVIAL_BOUNDS, span, |lint| {
                        lint.build(&format!(
                            "{} bound {} does not depend on any type \
                                or lifetime parameters",
                            predicate_kind_name, predicate
                        ))
                        .emit()
                    });
                }
            }
        }
    }
}

declare_lint_pass!(
    /// Does nothing as a lint pass, but registers some `Lint`s
    /// which are used by other parts of the compiler.
    SoftLints => [
        WHILE_TRUE,
        BOX_POINTERS,
        NON_SHORTHAND_FIELD_PATTERNS,
        UNSAFE_CODE,
        MISSING_DOCS,
        MISSING_COPY_IMPLEMENTATIONS,
        MISSING_DEBUG_IMPLEMENTATIONS,
        ANONYMOUS_PARAMETERS,
        UNUSED_DOC_COMMENTS,
        NO_MANGLE_CONST_ITEMS,
        NO_MANGLE_GENERIC_ITEMS,
        MUTABLE_TRANSMUTES,
        UNSTABLE_FEATURES,
        UNREACHABLE_PUB,
        TYPE_ALIAS_BOUNDS,
        TRIVIAL_BOUNDS
    ]
);

declare_lint! {
    /// The `ellipsis_inclusive_range_patterns` lint detects the [`...` range
    /// pattern], which is deprecated.
    ///
    /// [`...` range pattern]: https://doc.rust-lang.org/reference/patterns.html#range-patterns
    ///
    /// ### Example
    ///
    /// ```rust,edition2018
    /// let x = 123;
    /// match x {
    ///     0...100 => {}
    ///     _ => {}
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// The `...` range pattern syntax was changed to `..=` to avoid potential
    /// confusion with the [`..` range expression]. Use the new form instead.
    ///
    /// [`..` range expression]: https://doc.rust-lang.org/reference/expressions/range-expr.html
    pub ELLIPSIS_INCLUSIVE_RANGE_PATTERNS,
    Warn,
    "`...` range patterns are deprecated",
    @future_incompatible = FutureIncompatibleInfo {
        reference: "<https://doc.rust-lang.org/nightly/edition-guide/rust-2021/warnings-promoted-to-error.html>",
        reason: FutureIncompatibilityReason::EditionError(Edition::Edition2021),
    };
}

#[derive(Default)]
pub struct EllipsisInclusiveRangePatterns {
    /// If `Some(_)`, suppress all subsequent pattern
    /// warnings for better diagnostics.
    node_id: Option<ast::NodeId>,
}

impl_lint_pass!(EllipsisInclusiveRangePatterns => [ELLIPSIS_INCLUSIVE_RANGE_PATTERNS]);

impl EarlyLintPass for EllipsisInclusiveRangePatterns {
    fn check_pat(&mut self, cx: &EarlyContext<'_>, pat: &ast::Pat) {
        if self.node_id.is_some() {
            // Don't recursively warn about patterns inside range endpoints.
            return;
        }

        use self::ast::{PatKind, RangeSyntax::DotDotDot};

        /// If `pat` is a `...` pattern, return the start and end of the range, as well as the span
        /// corresponding to the ellipsis.
        fn matches_ellipsis_pat(pat: &ast::Pat) -> Option<(Option<&Expr>, &Expr, Span)> {
            match &pat.kind {
                PatKind::Range(
                    a,
                    Some(b),
                    Spanned { span, node: RangeEnd::Included(DotDotDot) },
                ) => Some((a.as_deref(), b, *span)),
                _ => None,
            }
        }

        let (parenthesise, endpoints) = match &pat.kind {
            PatKind::Ref(subpat, _) => (true, matches_ellipsis_pat(&subpat)),
            _ => (false, matches_ellipsis_pat(pat)),
        };

        if let Some((start, end, join)) = endpoints {
            let msg = "`...` range patterns are deprecated";
            let suggestion = "use `..=` for an inclusive range";
            if parenthesise {
                self.node_id = Some(pat.id);
                let end = expr_to_string(&end);
                let replace = match start {
                    Some(start) => format!("&({}..={})", expr_to_string(&start), end),
                    None => format!("&(..={})", end),
                };
                if join.edition() >= Edition::Edition2021 {
                    let mut err =
                        rustc_errors::struct_span_err!(cx.sess, pat.span, E0783, "{}", msg,);
                    err.span_suggestion(
                        pat.span,
                        suggestion,
                        replace,
                        Applicability::MachineApplicable,
                    )
                    .emit();
                } else {
                    cx.struct_span_lint(ELLIPSIS_INCLUSIVE_RANGE_PATTERNS, pat.span, |lint| {
                        lint.build(msg)
                            .span_suggestion(
                                pat.span,
                                suggestion,
                                replace,
                                Applicability::MachineApplicable,
                            )
                            .emit();
                    });
                }
            } else {
                let replace = "..=".to_owned();
                if join.edition() >= Edition::Edition2021 {
                    let mut err =
                        rustc_errors::struct_span_err!(cx.sess, pat.span, E0783, "{}", msg,);
                    err.span_suggestion_short(
                        join,
                        suggestion,
                        replace,
                        Applicability::MachineApplicable,
                    )
                    .emit();
                } else {
                    cx.struct_span_lint(ELLIPSIS_INCLUSIVE_RANGE_PATTERNS, join, |lint| {
                        lint.build(msg)
                            .span_suggestion_short(
                                join,
                                suggestion,
                                replace,
                                Applicability::MachineApplicable,
                            )
                            .emit();
                    });
                }
            };
        }
    }

    fn check_pat_post(&mut self, _cx: &EarlyContext<'_>, pat: &ast::Pat) {
        if let Some(node_id) = self.node_id {
            if pat.id == node_id {
                self.node_id = None
            }
        }
    }
}

declare_lint! {
    /// The `unnameable_test_items` lint detects [`#[test]`][test] functions
    /// that are not able to be run by the test harness because they are in a
    /// position where they are not nameable.
    ///
    /// [test]: https://doc.rust-lang.org/reference/attributes/testing.html#the-test-attribute
    ///
    /// ### Example
    ///
    /// ```rust,test
    /// fn main() {
    ///     #[test]
    ///     fn foo() {
    ///         // This test will not fail because it does not run.
    ///         assert_eq!(1, 2);
    ///     }
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// In order for the test harness to run a test, the test function must be
    /// located in a position where it can be accessed from the crate root.
    /// This generally means it must be defined in a module, and not anywhere
    /// else such as inside another function. The compiler previously allowed
    /// this without an error, so a lint was added as an alert that a test is
    /// not being used. Whether or not this should be allowed has not yet been
    /// decided, see [RFC 2471] and [issue #36629].
    ///
    /// [RFC 2471]: https://github.com/rust-lang/rfcs/pull/2471#issuecomment-397414443
    /// [issue #36629]: https://github.com/rust-lang/rust/issues/36629
    UNNAMEABLE_TEST_ITEMS,
    Warn,
    "detects an item that cannot be named being marked as `#[test_case]`",
    report_in_external_macro
}

pub struct UnnameableTestItems {
    boundary: Option<LocalDefId>, // Id of the item under which things are not nameable
    items_nameable: bool,
}

impl_lint_pass!(UnnameableTestItems => [UNNAMEABLE_TEST_ITEMS]);

impl UnnameableTestItems {
    pub fn new() -> Self {
        Self { boundary: None, items_nameable: true }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnnameableTestItems {
    fn check_item(&mut self, cx: &LateContext<'_>, it: &hir::Item<'_>) {
        if self.items_nameable {
            if let hir::ItemKind::Mod(..) = it.kind {
            } else {
                self.items_nameable = false;
                self.boundary = Some(it.def_id);
            }
            return;
        }

        let attrs = cx.tcx.hir().attrs(it.hir_id());
        if let Some(attr) = cx.sess().find_by_name(attrs, sym::rustc_test_marker) {
            cx.struct_span_lint(UNNAMEABLE_TEST_ITEMS, attr.span, |lint| {
                lint.build("cannot test inner items").emit()
            });
        }
    }

    fn check_item_post(&mut self, _cx: &LateContext<'_>, it: &hir::Item<'_>) {
        if !self.items_nameable && self.boundary == Some(it.def_id) {
            self.items_nameable = true;
        }
    }
}

declare_lint! {
    /// The `keyword_idents` lint detects edition keywords being used as an
    /// identifier.
    ///
    /// ### Example
    ///
    /// ```rust,edition2015,compile_fail
    /// #![deny(keyword_idents)]
    /// // edition 2015
    /// fn dyn() {}
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Rust [editions] allow the language to evolve without breaking
    /// backwards compatibility. This lint catches code that uses new keywords
    /// that are added to the language that are used as identifiers (such as a
    /// variable name, function name, etc.). If you switch the compiler to a
    /// new edition without updating the code, then it will fail to compile if
    /// you are using a new keyword as an identifier.
    ///
    /// You can manually change the identifiers to a non-keyword, or use a
    /// [raw identifier], for example `r#dyn`, to transition to a new edition.
    ///
    /// This lint solves the problem automatically. It is "allow" by default
    /// because the code is perfectly valid in older editions. The [`cargo
    /// fix`] tool with the `--edition` flag will switch this lint to "warn"
    /// and automatically apply the suggested fix from the compiler (which is
    /// to use a raw identifier). This provides a completely automated way to
    /// update old code for a new edition.
    ///
    /// [editions]: https://doc.rust-lang.org/edition-guide/
    /// [raw identifier]: https://doc.rust-lang.org/reference/identifiers.html
    /// [`cargo fix`]: https://doc.rust-lang.org/cargo/commands/cargo-fix.html
    pub KEYWORD_IDENTS,
    Allow,
    "detects edition keywords being used as an identifier",
    @future_incompatible = FutureIncompatibleInfo {
        reference: "issue #49716 <https://github.com/rust-lang/rust/issues/49716>",
        reason: FutureIncompatibilityReason::EditionError(Edition::Edition2018),
    };
}

declare_lint_pass!(
    /// Check for uses of edition keywords used as an identifier.
    KeywordIdents => [KEYWORD_IDENTS]
);

struct UnderMacro(bool);

impl KeywordIdents {
    fn check_tokens(&mut self, cx: &EarlyContext<'_>, tokens: TokenStream) {
        for tt in tokens.into_trees() {
            match tt {
                // Only report non-raw idents.
                TokenTree::Token(token) => {
                    if let Some((ident, false)) = token.ident() {
                        self.check_ident_token(cx, UnderMacro(true), ident);
                    }
                }
                TokenTree::Delimited(_, _, tts) => self.check_tokens(cx, tts),
            }
        }
    }

    fn check_ident_token(
        &mut self,
        cx: &EarlyContext<'_>,
        UnderMacro(under_macro): UnderMacro,
        ident: Ident,
    ) {
        let next_edition = match cx.sess.edition() {
            Edition::Edition2015 => {
                match ident.name {
                    kw::Async | kw::Await | kw::Try => Edition::Edition2018,

                    // rust-lang/rust#56327: Conservatively do not
                    // attempt to report occurrences of `dyn` within
                    // macro definitions or invocations, because `dyn`
                    // can legitimately occur as a contextual keyword
                    // in 2015 code denoting its 2018 meaning, and we
                    // do not want rustfix to inject bugs into working
                    // code by rewriting such occurrences.
                    //
                    // But if we see `dyn` outside of a macro, we know
                    // its precise role in the parsed AST and thus are
                    // assured this is truly an attempt to use it as
                    // an identifier.
                    kw::Dyn if !under_macro => Edition::Edition2018,

                    _ => return,
                }
            }

            // There are no new keywords yet for the 2018 edition and beyond.
            _ => return,
        };

        // Don't lint `r#foo`.
        if cx.sess.parse_sess.raw_identifier_spans.borrow().contains(&ident.span) {
            return;
        }

        cx.struct_span_lint(KEYWORD_IDENTS, ident.span, |lint| {
            lint.build(&format!("`{}` is a keyword in the {} edition", ident, next_edition))
                .span_suggestion(
                    ident.span,
                    "you can use a raw identifier to stay compatible",
                    format!("r#{}", ident),
                    Applicability::MachineApplicable,
                )
                .emit()
        });
    }
}

impl EarlyLintPass for KeywordIdents {
    fn check_mac_def(&mut self, cx: &EarlyContext<'_>, mac_def: &ast::MacroDef, _id: ast::NodeId) {
        self.check_tokens(cx, mac_def.body.inner_tokens());
    }
    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &ast::MacCall) {
        self.check_tokens(cx, mac.args.inner_tokens());
    }
    fn check_ident(&mut self, cx: &EarlyContext<'_>, ident: Ident) {
        self.check_ident_token(cx, UnderMacro(false), ident);
    }
}

declare_lint_pass!(ExplicitOutlivesRequirements => [EXPLICIT_OUTLIVES_REQUIREMENTS]);

impl ExplicitOutlivesRequirements {
    fn lifetimes_outliving_lifetime<'tcx>(
        inferred_outlives: &'tcx [(ty::Predicate<'tcx>, Span)],
        index: u32,
    ) -> Vec<ty::Region<'tcx>> {
        inferred_outlives
            .iter()
            .filter_map(|(pred, _)| match pred.kind().skip_binder() {
                ty::PredicateKind::RegionOutlives(ty::OutlivesPredicate(a, b)) => match a {
                    ty::ReEarlyBound(ebr) if ebr.index == index => Some(b),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    fn lifetimes_outliving_type<'tcx>(
        inferred_outlives: &'tcx [(ty::Predicate<'tcx>, Span)],
        index: u32,
    ) -> Vec<ty::Region<'tcx>> {
        inferred_outlives
            .iter()
            .filter_map(|(pred, _)| match pred.kind().skip_binder() {
                ty::PredicateKind::TypeOutlives(ty::OutlivesPredicate(a, b)) => {
                    a.is_param(index).then_some(b)
                }
                _ => None,
            })
            .collect()
    }

    fn collect_outlived_lifetimes<'tcx>(
        &self,
        param: &'tcx hir::GenericParam<'tcx>,
        tcx: TyCtxt<'tcx>,
        inferred_outlives: &'tcx [(ty::Predicate<'tcx>, Span)],
        ty_generics: &'tcx ty::Generics,
    ) -> Vec<ty::Region<'tcx>> {
        let index =
            ty_generics.param_def_id_to_index[&tcx.hir().local_def_id(param.hir_id).to_def_id()];

        match param.kind {
            hir::GenericParamKind::Lifetime { .. } => {
                Self::lifetimes_outliving_lifetime(inferred_outlives, index)
            }
            hir::GenericParamKind::Type { .. } => {
                Self::lifetimes_outliving_type(inferred_outlives, index)
            }
            hir::GenericParamKind::Const { .. } => Vec::new(),
        }
    }

    fn collect_outlives_bound_spans<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        bounds: &hir::GenericBounds<'_>,
        inferred_outlives: &[ty::Region<'tcx>],
        infer_static: bool,
    ) -> Vec<(usize, Span)> {
        use rustc_middle::middle::resolve_lifetime::Region;

        bounds
            .iter()
            .enumerate()
            .filter_map(|(i, bound)| {
                if let hir::GenericBound::Outlives(lifetime) = bound {
                    let is_inferred = match tcx.named_region(lifetime.hir_id) {
                        Some(Region::Static) if infer_static => {
                            inferred_outlives.iter().any(|r| matches!(r, ty::ReStatic))
                        }
                        Some(Region::EarlyBound(index, ..)) => inferred_outlives.iter().any(|r| {
                            if let ty::ReEarlyBound(ebr) = r { ebr.index == index } else { false }
                        }),
                        _ => false,
                    };
                    is_inferred.then_some((i, bound.span()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn consolidate_outlives_bound_spans(
        &self,
        lo: Span,
        bounds: &hir::GenericBounds<'_>,
        bound_spans: Vec<(usize, Span)>,
    ) -> Vec<Span> {
        if bounds.is_empty() {
            return Vec::new();
        }
        if bound_spans.len() == bounds.len() {
            let (_, last_bound_span) = bound_spans[bound_spans.len() - 1];
            // If all bounds are inferable, we want to delete the colon, so
            // start from just after the parameter (span passed as argument)
            vec![lo.to(last_bound_span)]
        } else {
            let mut merged = Vec::new();
            let mut last_merged_i = None;

            let mut from_start = true;
            for (i, bound_span) in bound_spans {
                match last_merged_i {
                    // If the first bound is inferable, our span should also eat the leading `+`.
                    None if i == 0 => {
                        merged.push(bound_span.to(bounds[1].span().shrink_to_lo()));
                        last_merged_i = Some(0);
                    }
                    // If consecutive bounds are inferable, merge their spans
                    Some(h) if i == h + 1 => {
                        if let Some(tail) = merged.last_mut() {
                            // Also eat the trailing `+` if the first
                            // more-than-one bound is inferable
                            let to_span = if from_start && i < bounds.len() {
                                bounds[i + 1].span().shrink_to_lo()
                            } else {
                                bound_span
                            };
                            *tail = tail.to(to_span);
                            last_merged_i = Some(i);
                        } else {
                            bug!("another bound-span visited earlier");
                        }
                    }
                    _ => {
                        // When we find a non-inferable bound, subsequent inferable bounds
                        // won't be consecutive from the start (and we'll eat the leading
                        // `+` rather than the trailing one)
                        from_start = false;
                        merged.push(bounds[i - 1].span().shrink_to_hi().to(bound_span));
                        last_merged_i = Some(i);
                    }
                }
            }
            merged
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ExplicitOutlivesRequirements {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'_>) {
        use rustc_middle::middle::resolve_lifetime::Region;

        let infer_static = cx.tcx.features().infer_static_outlives_requirements;
        let def_id = item.def_id;
        if let hir::ItemKind::Struct(_, ref hir_generics)
        | hir::ItemKind::Enum(_, ref hir_generics)
        | hir::ItemKind::Union(_, ref hir_generics) = item.kind
        {
            let inferred_outlives = cx.tcx.inferred_outlives_of(def_id);
            if inferred_outlives.is_empty() {
                return;
            }

            let ty_generics = cx.tcx.generics_of(def_id);

            let mut bound_count = 0;
            let mut lint_spans = Vec::new();

            for param in hir_generics.params {
                let has_lifetime_bounds = param
                    .bounds
                    .iter()
                    .any(|bound| matches!(bound, hir::GenericBound::Outlives(_)));
                if !has_lifetime_bounds {
                    continue;
                }

                let relevant_lifetimes =
                    self.collect_outlived_lifetimes(param, cx.tcx, inferred_outlives, ty_generics);
                if relevant_lifetimes.is_empty() {
                    continue;
                }

                let bound_spans = self.collect_outlives_bound_spans(
                    cx.tcx,
                    &param.bounds,
                    &relevant_lifetimes,
                    infer_static,
                );
                bound_count += bound_spans.len();
                lint_spans.extend(self.consolidate_outlives_bound_spans(
                    param.span.shrink_to_hi(),
                    &param.bounds,
                    bound_spans,
                ));
            }

            let mut where_lint_spans = Vec::new();
            let mut dropped_predicate_count = 0;
            let num_predicates = hir_generics.where_clause.predicates.len();
            for (i, where_predicate) in hir_generics.where_clause.predicates.iter().enumerate() {
                let (relevant_lifetimes, bounds, span) = match where_predicate {
                    hir::WherePredicate::RegionPredicate(predicate) => {
                        if let Some(Region::EarlyBound(index, ..)) =
                            cx.tcx.named_region(predicate.lifetime.hir_id)
                        {
                            (
                                Self::lifetimes_outliving_lifetime(inferred_outlives, index),
                                &predicate.bounds,
                                predicate.span,
                            )
                        } else {
                            continue;
                        }
                    }
                    hir::WherePredicate::BoundPredicate(predicate) => {
                        // FIXME we can also infer bounds on associated types,
                        // and should check for them here.
                        match predicate.bounded_ty.kind {
                            hir::TyKind::Path(hir::QPath::Resolved(None, ref path)) => {
                                let Res::Def(DefKind::TyParam, def_id) = path.res else {
                                    continue
                                };
                                let index = ty_generics.param_def_id_to_index[&def_id];
                                (
                                    Self::lifetimes_outliving_type(inferred_outlives, index),
                                    &predicate.bounds,
                                    predicate.span,
                                )
                            }
                            _ => {
                                continue;
                            }
                        }
                    }
                    _ => continue,
                };
                if relevant_lifetimes.is_empty() {
                    continue;
                }

                let bound_spans = self.collect_outlives_bound_spans(
                    cx.tcx,
                    bounds,
                    &relevant_lifetimes,
                    infer_static,
                );
                bound_count += bound_spans.len();

                let drop_predicate = bound_spans.len() == bounds.len();
                if drop_predicate {
                    dropped_predicate_count += 1;
                }

                // If all the bounds on a predicate were inferable and there are
                // further predicates, we want to eat the trailing comma.
                if drop_predicate && i + 1 < num_predicates {
                    let next_predicate_span = hir_generics.where_clause.predicates[i + 1].span();
                    where_lint_spans.push(span.to(next_predicate_span.shrink_to_lo()));
                } else {
                    where_lint_spans.extend(self.consolidate_outlives_bound_spans(
                        span.shrink_to_lo(),
                        bounds,
                        bound_spans,
                    ));
                }
            }

            // If all predicates are inferable, drop the entire clause
            // (including the `where`)
            if num_predicates > 0 && dropped_predicate_count == num_predicates {
                let where_span = hir_generics
                    .where_clause
                    .span()
                    .expect("span of (nonempty) where clause should exist");
                // Extend the where clause back to the closing `>` of the
                // generics, except for tuple struct, which have the `where`
                // after the fields of the struct.
                let full_where_span =
                    if let hir::ItemKind::Struct(hir::VariantData::Tuple(..), _) = item.kind {
                        where_span
                    } else {
                        hir_generics.span.shrink_to_hi().to(where_span)
                    };
                lint_spans.push(full_where_span);
            } else {
                lint_spans.extend(where_lint_spans);
            }

            if !lint_spans.is_empty() {
                cx.struct_span_lint(EXPLICIT_OUTLIVES_REQUIREMENTS, lint_spans.clone(), |lint| {
                    lint.build("outlives requirements can be inferred")
                        .multipart_suggestion(
                            if bound_count == 1 {
                                "remove this bound"
                            } else {
                                "remove these bounds"
                            },
                            lint_spans
                                .into_iter()
                                .map(|span| (span, "".to_owned()))
                                .collect::<Vec<_>>(),
                            Applicability::MachineApplicable,
                        )
                        .emit();
                });
            }
        }
    }
}

declare_lint! {
    /// The `incomplete_features` lint detects unstable features enabled with
    /// the [`feature` attribute] that may function improperly in some or all
    /// cases.
    ///
    /// [`feature` attribute]: https://doc.rust-lang.org/nightly/unstable-book/
    ///
    /// ### Example
    ///
    /// ```rust
    /// #![feature(generic_const_exprs)]
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Although it is encouraged for people to experiment with unstable
    /// features, some of them are known to be incomplete or faulty. This lint
    /// is a signal that the feature has not yet been finished, and you may
    /// experience problems with it.
    pub INCOMPLETE_FEATURES,
    Warn,
    "incomplete features that may function improperly in some or all cases"
}

declare_lint_pass!(
    /// Check for used feature gates in `INCOMPLETE_FEATURES` in `rustc_feature/src/active.rs`.
    IncompleteFeatures => [INCOMPLETE_FEATURES]
);

impl EarlyLintPass for IncompleteFeatures {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _: &ast::Crate) {
        let features = cx.sess.features_untracked();
        features
            .declared_lang_features
            .iter()
            .map(|(name, span, _)| (name, span))
            .chain(features.declared_lib_features.iter().map(|(name, span)| (name, span)))
            .filter(|(&name, _)| features.incomplete(name))
            .for_each(|(&name, &span)| {
                cx.struct_span_lint(INCOMPLETE_FEATURES, span, |lint| {
                    let mut builder = lint.build(&format!(
                        "the feature `{}` is incomplete and may not be safe to use \
                         and/or cause compiler crashes",
                        name,
                    ));
                    if let Some(n) = rustc_feature::find_feature_issue(name, GateIssue::Language) {
                        builder.note(&format!(
                            "see issue #{} <https://github.com/rust-lang/rust/issues/{}> \
                             for more information",
                            n, n,
                        ));
                    }
                    if HAS_MIN_FEATURES.contains(&name) {
                        builder.help(&format!(
                            "consider using `min_{}` instead, which is more stable and complete",
                            name,
                        ));
                    }
                    builder.emit();
                })
            });
    }
}

const HAS_MIN_FEATURES: &[Symbol] = &[sym::specialization];

declare_lint! {
    /// The `invalid_value` lint detects creating a value that is not valid,
    /// such as a null reference.
    ///
    /// ### Example
    ///
    /// ```rust,no_run
    /// # #![allow(unused)]
    /// unsafe {
    ///     let x: &'static i32 = std::mem::zeroed();
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// In some situations the compiler can detect that the code is creating
    /// an invalid value, which should be avoided.
    ///
    /// In particular, this lint will check for improper use of
    /// [`mem::zeroed`], [`mem::uninitialized`], [`mem::transmute`], and
    /// [`MaybeUninit::assume_init`] that can cause [undefined behavior]. The
    /// lint should provide extra information to indicate what the problem is
    /// and a possible solution.
    ///
    /// [`mem::zeroed`]: https://doc.rust-lang.org/std/mem/fn.zeroed.html
    /// [`mem::uninitialized`]: https://doc.rust-lang.org/std/mem/fn.uninitialized.html
    /// [`mem::transmute`]: https://doc.rust-lang.org/std/mem/fn.transmute.html
    /// [`MaybeUninit::assume_init`]: https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#method.assume_init
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    pub INVALID_VALUE,
    Warn,
    "an invalid value is being created (such as a null reference)"
}

declare_lint_pass!(InvalidValue => [INVALID_VALUE]);

impl<'tcx> LateLintPass<'tcx> for InvalidValue {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr<'_>) {
        #[derive(Debug, Copy, Clone, PartialEq)]
        enum InitKind {
            Zeroed,
            Uninit,
        }

        /// Information about why a type cannot be initialized this way.
        /// Contains an error message and optionally a span to point at.
        type InitError = (String, Option<Span>);

        /// Test if this constant is all-0.
        fn is_zero(expr: &hir::Expr<'_>) -> bool {
            use hir::ExprKind::*;
            use rustc_ast::LitKind::*;
            match &expr.kind {
                Lit(lit) => {
                    if let Int(i, _) = lit.node {
                        i == 0
                    } else {
                        false
                    }
                }
                Tup(tup) => tup.iter().all(is_zero),
                _ => false,
            }
        }

        /// Determine if this expression is a "dangerous initialization".
        fn is_dangerous_init(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> Option<InitKind> {
            if let hir::ExprKind::Call(ref path_expr, ref args) = expr.kind {
                // Find calls to `mem::{uninitialized,zeroed}` methods.
                if let hir::ExprKind::Path(ref qpath) = path_expr.kind {
                    let def_id = cx.qpath_res(qpath, path_expr.hir_id).opt_def_id()?;
                    match cx.tcx.get_diagnostic_name(def_id) {
                        Some(sym::mem_zeroed) => return Some(InitKind::Zeroed),
                        Some(sym::mem_uninitialized) => return Some(InitKind::Uninit),
                        Some(sym::transmute) if is_zero(&args[0]) => return Some(InitKind::Zeroed),
                        _ => {}
                    }
                }
            } else if let hir::ExprKind::MethodCall(_, _, ref args, _) = expr.kind {
                // Find problematic calls to `MaybeUninit::assume_init`.
                let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
                if cx.tcx.is_diagnostic_item(sym::assume_init, def_id) {
                    // This is a call to *some* method named `assume_init`.
                    // See if the `self` parameter is one of the dangerous constructors.
                    if let hir::ExprKind::Call(ref path_expr, _) = args[0].kind {
                        if let hir::ExprKind::Path(ref qpath) = path_expr.kind {
                            let def_id = cx.qpath_res(qpath, path_expr.hir_id).opt_def_id()?;
                            match cx.tcx.get_diagnostic_name(def_id) {
                                Some(sym::maybe_uninit_zeroed) => return Some(InitKind::Zeroed),
                                Some(sym::maybe_uninit_uninit) => return Some(InitKind::Uninit),
                                _ => {}
                            }
                        }
                    }
                }
            }

            None
        }

        /// Test if this enum has several actually "existing" variants.
        /// Zero-sized uninhabited variants do not always have a tag assigned and thus do not "exist".
        fn is_multi_variant(adt: &ty::AdtDef) -> bool {
            // As an approximation, we only count dataless variants. Those are definitely inhabited.
            let existing_variants = adt.variants.iter().filter(|v| v.fields.is_empty()).count();
            existing_variants > 1
        }

        /// Return `Some` only if we are sure this type does *not*
        /// allow zero initialization.
        fn ty_find_init_error<'tcx>(
            tcx: TyCtxt<'tcx>,
            ty: Ty<'tcx>,
            init: InitKind,
        ) -> Option<InitError> {
            use rustc_middle::ty::TyKind::*;
            match ty.kind() {
                // Primitive types that don't like 0 as a value.
                Ref(..) => Some(("references must be non-null".to_string(), None)),
                Adt(..) if ty.is_box() => Some(("`Box` must be non-null".to_string(), None)),
                FnPtr(..) => Some(("function pointers must be non-null".to_string(), None)),
                Never => Some(("the `!` type has no valid value".to_string(), None)),
                RawPtr(tm) if matches!(tm.ty.kind(), Dynamic(..)) =>
                // raw ptr to dyn Trait
                {
                    Some(("the vtable of a wide raw pointer must be non-null".to_string(), None))
                }
                // Primitive types with other constraints.
                Bool if init == InitKind::Uninit => {
                    Some(("booleans must be either `true` or `false`".to_string(), None))
                }
                Char if init == InitKind::Uninit => {
                    Some(("characters must be a valid Unicode codepoint".to_string(), None))
                }
                // Recurse and checks for some compound types.
                Adt(adt_def, substs) if !adt_def.is_union() => {
                    // First check if this ADT has a layout attribute (like `NonNull` and friends).
                    use std::ops::Bound;
                    match tcx.layout_scalar_valid_range(adt_def.did) {
                        // We exploit here that `layout_scalar_valid_range` will never
                        // return `Bound::Excluded`.  (And we have tests checking that we
                        // handle the attribute correctly.)
                        (Bound::Included(lo), _) if lo > 0 => {
                            return Some((format!("`{}` must be non-null", ty), None));
                        }
                        (Bound::Included(_), _) | (_, Bound::Included(_))
                            if init == InitKind::Uninit =>
                        {
                            return Some((
                                format!(
                                    "`{}` must be initialized inside its custom valid range",
                                    ty,
                                ),
                                None,
                            ));
                        }
                        _ => {}
                    }
                    // Now, recurse.
                    match adt_def.variants.len() {
                        0 => Some(("enums with no variants have no valid value".to_string(), None)),
                        1 => {
                            // Struct, or enum with exactly one variant.
                            // Proceed recursively, check all fields.
                            let variant = &adt_def.variants[VariantIdx::from_u32(0)];
                            variant.fields.iter().find_map(|field| {
                                ty_find_init_error(tcx, field.ty(tcx, substs), init).map(
                                    |(mut msg, span)| {
                                        if span.is_none() {
                                            // Point to this field, should be helpful for figuring
                                            // out where the source of the error is.
                                            let span = tcx.def_span(field.did);
                                            write!(
                                                &mut msg,
                                                " (in this {} field)",
                                                adt_def.descr()
                                            )
                                            .unwrap();
                                            (msg, Some(span))
                                        } else {
                                            // Just forward.
                                            (msg, span)
                                        }
                                    },
                                )
                            })
                        }
                        // Multi-variant enum.
                        _ => {
                            if init == InitKind::Uninit && is_multi_variant(adt_def) {
                                let span = tcx.def_span(adt_def.did);
                                Some((
                                    "enums have to be initialized to a variant".to_string(),
                                    Some(span),
                                ))
                            } else {
                                // In principle, for zero-initialization we could figure out which variant corresponds
                                // to tag 0, and check that... but for now we just accept all zero-initializations.
                                None
                            }
                        }
                    }
                }
                Tuple(..) => {
                    // Proceed recursively, check all fields.
                    ty.tuple_fields().find_map(|field| ty_find_init_error(tcx, field, init))
                }
                // Conservative fallback.
                _ => None,
            }
        }

        if let Some(init) = is_dangerous_init(cx, expr) {
            // This conjures an instance of a type out of nothing,
            // using zeroed or uninitialized memory.
            // We are extremely conservative with what we warn about.
            let conjured_ty = cx.typeck_results().expr_ty(expr);
            if let Some((msg, span)) =
                with_no_trimmed_paths(|| ty_find_init_error(cx.tcx, conjured_ty, init))
            {
                cx.struct_span_lint(INVALID_VALUE, expr.span, |lint| {
                    let mut err = lint.build(&format!(
                        "the type `{}` does not permit {}",
                        conjured_ty,
                        match init {
                            InitKind::Zeroed => "zero-initialization",
                            InitKind::Uninit => "being left uninitialized",
                        },
                    ));
                    err.span_label(expr.span, "this code causes undefined behavior when executed");
                    err.span_label(
                        expr.span,
                        "help: use `MaybeUninit<T>` instead, \
                            and only call `assume_init` after initialization is done",
                    );
                    if let Some(span) = span {
                        err.span_note(span, &msg);
                    } else {
                        err.note(&msg);
                    }
                    err.emit();
                });
            }
        }
    }
}

declare_lint! {
    /// The `clashing_extern_declarations` lint detects when an `extern fn`
    /// has been declared with the same name but different types.
    ///
    /// ### Example
    ///
    /// ```rust
    /// mod m {
    ///     extern "C" {
    ///         fn foo();
    ///     }
    /// }
    ///
    /// extern "C" {
    ///     fn foo(_: u32);
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Because two symbols of the same name cannot be resolved to two
    /// different functions at link time, and one function cannot possibly
    /// have two types, a clashing extern declaration is almost certainly a
    /// mistake. Check to make sure that the `extern` definitions are correct
    /// and equivalent, and possibly consider unifying them in one location.
    ///
    /// This lint does not run between crates because a project may have
    /// dependencies which both rely on the same extern function, but declare
    /// it in a different (but valid) way. For example, they may both declare
    /// an opaque type for one or more of the arguments (which would end up
    /// distinct types), or use types that are valid conversions in the
    /// language the `extern fn` is defined in. In these cases, the compiler
    /// can't say that the clashing declaration is incorrect.
    pub CLASHING_EXTERN_DECLARATIONS,
    Warn,
    "detects when an extern fn has been declared with the same name but different types"
}

pub struct ClashingExternDeclarations {
    /// Map of function symbol name to the first-seen hir id for that symbol name.. If seen_decls
    /// contains an entry for key K, it means a symbol with name K has been seen by this lint and
    /// the symbol should be reported as a clashing declaration.
    // FIXME: Technically, we could just store a &'tcx str here without issue; however, the
    // `impl_lint_pass` macro doesn't currently support lints parametric over a lifetime.
    seen_decls: FxHashMap<Symbol, HirId>,
}

/// Differentiate between whether the name for an extern decl came from the link_name attribute or
/// just from declaration itself. This is important because we don't want to report clashes on
/// symbol name if they don't actually clash because one or the other links against a symbol with a
/// different name.
enum SymbolName {
    /// The name of the symbol + the span of the annotation which introduced the link name.
    Link(Symbol, Span),
    /// No link name, so just the name of the symbol.
    Normal(Symbol),
}

impl SymbolName {
    fn get_name(&self) -> Symbol {
        match self {
            SymbolName::Link(s, _) | SymbolName::Normal(s) => *s,
        }
    }
}

impl ClashingExternDeclarations {
    crate fn new() -> Self {
        ClashingExternDeclarations { seen_decls: FxHashMap::default() }
    }
    /// Insert a new foreign item into the seen set. If a symbol with the same name already exists
    /// for the item, return its HirId without updating the set.
    fn insert(&mut self, tcx: TyCtxt<'_>, fi: &hir::ForeignItem<'_>) -> Option<HirId> {
        let did = fi.def_id.to_def_id();
        let instance = Instance::new(did, ty::List::identity_for_item(tcx, did));
        let name = Symbol::intern(tcx.symbol_name(instance).name);
        if let Some(&hir_id) = self.seen_decls.get(&name) {
            // Avoid updating the map with the new entry when we do find a collision. We want to
            // make sure we're always pointing to the first definition as the previous declaration.
            // This lets us avoid emitting "knock-on" diagnostics.
            Some(hir_id)
        } else {
            self.seen_decls.insert(name, fi.hir_id())
        }
    }

    /// Get the name of the symbol that's linked against for a given extern declaration. That is,
    /// the name specified in a #[link_name = ...] attribute if one was specified, else, just the
    /// symbol's name.
    fn name_of_extern_decl(tcx: TyCtxt<'_>, fi: &hir::ForeignItem<'_>) -> SymbolName {
        if let Some((overridden_link_name, overridden_link_name_span)) =
            tcx.codegen_fn_attrs(fi.def_id).link_name.map(|overridden_link_name| {
                // FIXME: Instead of searching through the attributes again to get span
                // information, we could have codegen_fn_attrs also give span information back for
                // where the attribute was defined. However, until this is found to be a
                // bottleneck, this does just fine.
                (
                    overridden_link_name,
                    tcx.get_attrs(fi.def_id.to_def_id())
                        .iter()
                        .find(|at| at.has_name(sym::link_name))
                        .unwrap()
                        .span,
                )
            })
        {
            SymbolName::Link(overridden_link_name, overridden_link_name_span)
        } else {
            SymbolName::Normal(fi.ident.name)
        }
    }

    /// Checks whether two types are structurally the same enough that the declarations shouldn't
    /// clash. We need this so we don't emit a lint when two modules both declare an extern struct,
    /// with the same members (as the declarations shouldn't clash).
    fn structurally_same_type<'tcx>(
        cx: &LateContext<'tcx>,
        a: Ty<'tcx>,
        b: Ty<'tcx>,
        ckind: CItemKind,
    ) -> bool {
        fn structurally_same_type_impl<'tcx>(
            seen_types: &mut FxHashSet<(Ty<'tcx>, Ty<'tcx>)>,
            cx: &LateContext<'tcx>,
            a: Ty<'tcx>,
            b: Ty<'tcx>,
            ckind: CItemKind,
        ) -> bool {
            debug!("structurally_same_type_impl(cx, a = {:?}, b = {:?})", a, b);
            let tcx = cx.tcx;

            // Given a transparent newtype, reach through and grab the inner
            // type unless the newtype makes the type non-null.
            let non_transparent_ty = |ty: Ty<'tcx>| -> Ty<'tcx> {
                let mut ty = ty;
                loop {
                    if let ty::Adt(def, substs) = *ty.kind() {
                        let is_transparent = def.subst(tcx, substs).repr.transparent();
                        let is_non_null = crate::types::nonnull_optimization_guaranteed(tcx, &def);
                        debug!(
                            "non_transparent_ty({:?}) -- type is transparent? {}, type is non-null? {}",
                            ty, is_transparent, is_non_null
                        );
                        if is_transparent && !is_non_null {
                            debug_assert!(def.variants.len() == 1);
                            let v = &def.variants[VariantIdx::new(0)];
                            ty = transparent_newtype_field(tcx, v)
                                .expect(
                                    "single-variant transparent structure with zero-sized field",
                                )
                                .ty(tcx, substs);
                            continue;
                        }
                    }
                    debug!("non_transparent_ty -> {:?}", ty);
                    return ty;
                }
            };

            let a = non_transparent_ty(a);
            let b = non_transparent_ty(b);

            if !seen_types.insert((a, b)) {
                // We've encountered a cycle. There's no point going any further -- the types are
                // structurally the same.
                return true;
            }
            let tcx = cx.tcx;
            if a == b || rustc_middle::ty::TyS::same_type(a, b) {
                // All nominally-same types are structurally same, too.
                true
            } else {
                // Do a full, depth-first comparison between the two.
                use rustc_middle::ty::TyKind::*;
                let a_kind = a.kind();
                let b_kind = b.kind();

                let compare_layouts = |a, b| -> Result<bool, LayoutError<'tcx>> {
                    debug!("compare_layouts({:?}, {:?})", a, b);
                    let a_layout = &cx.layout_of(a)?.layout.abi;
                    let b_layout = &cx.layout_of(b)?.layout.abi;
                    debug!(
                        "comparing layouts: {:?} == {:?} = {}",
                        a_layout,
                        b_layout,
                        a_layout == b_layout
                    );
                    Ok(a_layout == b_layout)
                };

                #[allow(rustc::usage_of_ty_tykind)]
                let is_primitive_or_pointer = |kind: &ty::TyKind<'_>| {
                    kind.is_primitive() || matches!(kind, RawPtr(..) | Ref(..))
                };

                ensure_sufficient_stack(|| {
                    match (a_kind, b_kind) {
                        (Adt(a_def, a_substs), Adt(b_def, b_substs)) => {
                            let a = a.subst(cx.tcx, a_substs);
                            let b = b.subst(cx.tcx, b_substs);
                            debug!("Comparing {:?} and {:?}", a, b);

                            // We can immediately rule out these types as structurally same if
                            // their layouts differ.
                            match compare_layouts(a, b) {
                                Ok(false) => return false,
                                _ => (), // otherwise, continue onto the full, fields comparison
                            }

                            // Grab a flattened representation of all fields.
                            let a_fields = a_def.variants.iter().flat_map(|v| v.fields.iter());
                            let b_fields = b_def.variants.iter().flat_map(|v| v.fields.iter());

                            // Perform a structural comparison for each field.
                            a_fields.eq_by(
                                b_fields,
                                |&ty::FieldDef { did: a_did, .. },
                                 &ty::FieldDef { did: b_did, .. }| {
                                    structurally_same_type_impl(
                                        seen_types,
                                        cx,
                                        tcx.type_of(a_did),
                                        tcx.type_of(b_did),
                                        ckind,
                                    )
                                },
                            )
                        }
                        (Array(a_ty, a_const), Array(b_ty, b_const)) => {
                            // For arrays, we also check the constness of the type.
                            a_const.val == b_const.val
                                && structurally_same_type_impl(seen_types, cx, a_ty, b_ty, ckind)
                        }
                        (Slice(a_ty), Slice(b_ty)) => {
                            structurally_same_type_impl(seen_types, cx, a_ty, b_ty, ckind)
                        }
                        (RawPtr(a_tymut), RawPtr(b_tymut)) => {
                            a_tymut.mutbl == b_tymut.mutbl
                                && structurally_same_type_impl(
                                    seen_types,
                                    cx,
                                    &a_tymut.ty,
                                    &b_tymut.ty,
                                    ckind,
                                )
                        }
                        (Ref(_a_region, a_ty, a_mut), Ref(_b_region, b_ty, b_mut)) => {
                            // For structural sameness, we don't need the region to be same.
                            a_mut == b_mut
                                && structurally_same_type_impl(seen_types, cx, a_ty, b_ty, ckind)
                        }
                        (FnDef(..), FnDef(..)) => {
                            let a_poly_sig = a.fn_sig(tcx);
                            let b_poly_sig = b.fn_sig(tcx);

                            // As we don't compare regions, skip_binder is fine.
                            let a_sig = a_poly_sig.skip_binder();
                            let b_sig = b_poly_sig.skip_binder();

                            (a_sig.abi, a_sig.unsafety, a_sig.c_variadic)
                                == (b_sig.abi, b_sig.unsafety, b_sig.c_variadic)
                                && a_sig.inputs().iter().eq_by(b_sig.inputs().iter(), |a, b| {
                                    structurally_same_type_impl(seen_types, cx, a, b, ckind)
                                })
                                && structurally_same_type_impl(
                                    seen_types,
                                    cx,
                                    a_sig.output(),
                                    b_sig.output(),
                                    ckind,
                                )
                        }
                        (Tuple(a_substs), Tuple(b_substs)) => {
                            a_substs.types().eq_by(b_substs.types(), |a_ty, b_ty| {
                                structurally_same_type_impl(seen_types, cx, a_ty, b_ty, ckind)
                            })
                        }
                        // For these, it's not quite as easy to define structural-sameness quite so easily.
                        // For the purposes of this lint, take the conservative approach and mark them as
                        // not structurally same.
                        (Dynamic(..), Dynamic(..))
                        | (Error(..), Error(..))
                        | (Closure(..), Closure(..))
                        | (Generator(..), Generator(..))
                        | (GeneratorWitness(..), GeneratorWitness(..))
                        | (Projection(..), Projection(..))
                        | (Opaque(..), Opaque(..)) => false,

                        // These definitely should have been caught above.
                        (Bool, Bool) | (Char, Char) | (Never, Never) | (Str, Str) => unreachable!(),

                        // An Adt and a primitive or pointer type. This can be FFI-safe if non-null
                        // enum layout optimisation is being applied.
                        (Adt(..), other_kind) | (other_kind, Adt(..))
                            if is_primitive_or_pointer(other_kind) =>
                        {
                            let (primitive, adt) =
                                if is_primitive_or_pointer(a.kind()) { (a, b) } else { (b, a) };
                            if let Some(ty) = crate::types::repr_nullable_ptr(cx, adt, ckind) {
                                ty == primitive
                            } else {
                                compare_layouts(a, b).unwrap_or(false)
                            }
                        }
                        // Otherwise, just compare the layouts. This may fail to lint for some
                        // incompatible types, but at the very least, will stop reads into
                        // uninitialised memory.
                        _ => compare_layouts(a, b).unwrap_or(false),
                    }
                })
            }
        }
        let mut seen_types = FxHashSet::default();
        structurally_same_type_impl(&mut seen_types, cx, a, b, ckind)
    }
}

impl_lint_pass!(ClashingExternDeclarations => [CLASHING_EXTERN_DECLARATIONS]);

impl<'tcx> LateLintPass<'tcx> for ClashingExternDeclarations {
    fn check_foreign_item(&mut self, cx: &LateContext<'tcx>, this_fi: &hir::ForeignItem<'_>) {
        trace!("ClashingExternDeclarations: check_foreign_item: {:?}", this_fi);
        if let ForeignItemKind::Fn(..) = this_fi.kind {
            let tcx = cx.tcx;
            if let Some(existing_hid) = self.insert(tcx, this_fi) {
                let existing_decl_ty = tcx.type_of(tcx.hir().local_def_id(existing_hid));
                let this_decl_ty = tcx.type_of(this_fi.def_id);
                debug!(
                    "ClashingExternDeclarations: Comparing existing {:?}: {:?} to this {:?}: {:?}",
                    existing_hid, existing_decl_ty, this_fi.def_id, this_decl_ty
                );
                // Check that the declarations match.
                if !Self::structurally_same_type(
                    cx,
                    existing_decl_ty,
                    this_decl_ty,
                    CItemKind::Declaration,
                ) {
                    let orig_fi = tcx.hir().expect_foreign_item(existing_hid.expect_owner());
                    let orig = Self::name_of_extern_decl(tcx, orig_fi);

                    // We want to ensure that we use spans for both decls that include where the
                    // name was defined, whether that was from the link_name attribute or not.
                    let get_relevant_span =
                        |fi: &hir::ForeignItem<'_>| match Self::name_of_extern_decl(tcx, fi) {
                            SymbolName::Normal(_) => fi.span,
                            SymbolName::Link(_, annot_span) => fi.span.to(annot_span),
                        };
                    // Finally, emit the diagnostic.
                    tcx.struct_span_lint_hir(
                        CLASHING_EXTERN_DECLARATIONS,
                        this_fi.hir_id(),
                        get_relevant_span(this_fi),
                        |lint| {
                            let mut expected_str = DiagnosticStyledString::new();
                            expected_str.push(existing_decl_ty.fn_sig(tcx).to_string(), false);
                            let mut found_str = DiagnosticStyledString::new();
                            found_str.push(this_decl_ty.fn_sig(tcx).to_string(), true);

                            lint.build(&format!(
                                "`{}` redeclare{} with a different signature",
                                this_fi.ident.name,
                                if orig.get_name() == this_fi.ident.name {
                                    "d".to_string()
                                } else {
                                    format!("s `{}`", orig.get_name())
                                }
                            ))
                            .span_label(
                                get_relevant_span(orig_fi),
                                &format!("`{}` previously declared here", orig.get_name()),
                            )
                            .span_label(
                                get_relevant_span(this_fi),
                                "this signature doesn't match the previous declaration",
                            )
                            .note_expected_found(&"", expected_str, &"", found_str)
                            .emit()
                        },
                    );
                }
            }
        }
    }
}

declare_lint! {
    /// The `deref_nullptr` lint detects when an null pointer is dereferenced,
    /// which causes [undefined behavior].
    ///
    /// ### Example
    ///
    /// ```rust,no_run
    /// # #![allow(unused)]
    /// use std::ptr;
    /// unsafe {
    ///     let x = &*ptr::null::<i32>();
    ///     let x = ptr::addr_of!(*ptr::null::<i32>());
    ///     let x = *(0 as *const i32);
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Dereferencing a null pointer causes [undefined behavior] even as a place expression,
    /// like `&*(0 as *const i32)` or `addr_of!(*(0 as *const i32))`.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    pub DEREF_NULLPTR,
    Warn,
    "detects when an null pointer is dereferenced"
}

declare_lint_pass!(DerefNullPtr => [DEREF_NULLPTR]);

impl<'tcx> LateLintPass<'tcx> for DerefNullPtr {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &hir::Expr<'_>) {
        /// test if expression is a null ptr
        fn is_null_ptr(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> bool {
            match &expr.kind {
                rustc_hir::ExprKind::Cast(ref expr, ref ty) => {
                    if let rustc_hir::TyKind::Ptr(_) = ty.kind {
                        return is_zero(expr) || is_null_ptr(cx, expr);
                    }
                }
                // check for call to `core::ptr::null` or `core::ptr::null_mut`
                rustc_hir::ExprKind::Call(ref path, _) => {
                    if let rustc_hir::ExprKind::Path(ref qpath) = path.kind {
                        if let Some(def_id) = cx.qpath_res(qpath, path.hir_id).opt_def_id() {
                            return matches!(
                                cx.tcx.get_diagnostic_name(def_id),
                                Some(sym::ptr_null | sym::ptr_null_mut)
                            );
                        }
                    }
                }
                _ => {}
            }
            false
        }

        /// test if expression is the literal `0`
        fn is_zero(expr: &hir::Expr<'_>) -> bool {
            match &expr.kind {
                rustc_hir::ExprKind::Lit(ref lit) => {
                    if let LitKind::Int(a, _) = lit.node {
                        return a == 0;
                    }
                }
                _ => {}
            }
            false
        }

        if let rustc_hir::ExprKind::Unary(rustc_hir::UnOp::Deref, expr_deref) = expr.kind {
            if is_null_ptr(cx, expr_deref) {
                cx.struct_span_lint(DEREF_NULLPTR, expr.span, |lint| {
                    let mut err = lint.build("dereferencing a null pointer");
                    err.span_label(expr.span, "this code causes undefined behavior when executed");
                    err.emit();
                });
            }
        }
    }
}

declare_lint! {
    /// The `named_asm_labels` lint detects the use of named labels in the
    /// inline `asm!` macro.
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// use std::arch::asm;
    ///
    /// fn main() {
    ///     unsafe {
    ///         asm!("foo: bar");
    ///     }
    /// }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// LLVM is allowed to duplicate inline assembly blocks for any
    /// reason, for example when it is in a function that gets inlined. Because
    /// of this, GNU assembler [local labels] *must* be used instead of labels
    /// with a name. Using named labels might cause assembler or linker errors.
    ///
    /// [local labels]: https://sourceware.org/binutils/docs/as/Symbol-Names.html#Local-Labels
    pub NAMED_ASM_LABELS,
    Deny,
    "named labels in inline assembly",
}

declare_lint_pass!(NamedAsmLabels => [NAMED_ASM_LABELS]);

impl<'tcx> LateLintPass<'tcx> for NamedAsmLabels {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::Expr {
            kind: hir::ExprKind::InlineAsm(hir::InlineAsm { template_strs, .. }),
            ..
        } = expr
        {
            for (template_sym, template_snippet, template_span) in template_strs.iter() {
                let template_str = template_sym.as_str();
                let find_label_span = |needle: &str| -> Option<Span> {
                    if let Some(template_snippet) = template_snippet {
                        let snippet = template_snippet.as_str();
                        if let Some(pos) = snippet.find(needle) {
                            let end = pos
                                + snippet[pos..]
                                    .find(|c| c == ':')
                                    .unwrap_or(snippet[pos..].len() - 1);
                            let inner = InnerSpan::new(pos, end);
                            return Some(template_span.from_inner(inner));
                        }
                    }

                    None
                };

                let mut found_labels = Vec::new();

                // A semicolon might not actually be specified as a separator for all targets, but it seems like LLVM accepts it always
                let statements = template_str.split(|c| matches!(c, '\n' | ';'));
                for statement in statements {
                    // If there's a comment, trim it from the statement
                    let statement = statement.find("//").map_or(statement, |idx| &statement[..idx]);
                    let mut start_idx = 0;
                    for (idx, _) in statement.match_indices(':') {
                        let possible_label = statement[start_idx..idx].trim();
                        let mut chars = possible_label.chars();
                        let Some(c) = chars.next() else {
                            // Empty string means a leading ':' in this section, which is not a label
                            break
                        };
                        // A label starts with an alphabetic character or . or _ and continues with alphanumeric characters, _, or $
                        if (c.is_alphabetic() || matches!(c, '.' | '_'))
                            && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '$'))
                        {
                            found_labels.push(possible_label);
                        } else {
                            // If we encounter a non-label, there cannot be any further labels, so stop checking
                            break;
                        }

                        start_idx = idx + 1;
                    }
                }

                debug!("NamedAsmLabels::check_expr(): found_labels: {:#?}", &found_labels);

                if found_labels.len() > 0 {
                    let spans = found_labels
                        .into_iter()
                        .filter_map(|label| find_label_span(label))
                        .collect::<Vec<Span>>();
                    // If there were labels but we couldn't find a span, combine the warnings and use the template span
                    let target_spans: MultiSpan =
                        if spans.len() > 0 { spans.into() } else { (*template_span).into() };

                    cx.lookup_with_diagnostics(
                            NAMED_ASM_LABELS,
                            Some(target_spans),
                            |diag| {
                                let mut err =
                                    diag.build("avoid using named labels in inline assembly");
                                err.emit();
                            },
                            BuiltinLintDiagnostics::NamedAsmLabel(
                                "only local labels of the form `<number>:` should be used in inline asm"
                                    .to_string(),
                            ),
                        );
                }
            }
        }
    }
}
