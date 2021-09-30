#![feature(plugin, rustc_private)]
#![crate_type = "dylib"]

extern crate rustc_ast_pretty;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_lint;
#[macro_use]
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_ast;

use rustc_ast_pretty::pprust;
use rustc_driver::plugin::Registry;
use rustc_hir as hir;
use rustc_hir::intravisit;
use rustc_hir::Node;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::source_map;

#[no_mangle]
fn __rustc_plugin_registrar(reg: &mut Registry) {
    reg.lint_store.register_lints(&[&MISSING_ALLOWED_ATTR]);
    reg.lint_store.register_late_pass(|| Box::new(MissingAllowedAttrPass));
}

declare_lint! {
    MISSING_ALLOWED_ATTR,
    Deny,
    "Checks for missing `allowed_attr` attribute"
}

declare_lint_pass!(MissingAllowedAttrPass => [MISSING_ALLOWED_ATTR]);

impl<'tcx> LateLintPass<'tcx> for MissingAllowedAttrPass {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: intravisit::FnKind<'tcx>,
        _: &'tcx hir::FnDecl,
        _: &'tcx hir::Body,
        span: source_map::Span,
        id: hir::HirId,
    ) {
        let item = match cx.tcx.hir().get(id) {
            Node::Item(item) => item,
            _ => cx.tcx.hir().expect_item(cx.tcx.hir().get_parent_item(id)),
        };

        let allowed = |attr| pprust::attribute_to_string(attr).contains("allowed_attr");
        if !cx.tcx.hir().attrs(item.hir_id()).iter().any(allowed) {
            cx.lint(MISSING_ALLOWED_ATTR, |lint| {
                lint.build("Missing 'allowed_attr' attribute").set_span(span).emit()
            });
        }
    }
}
