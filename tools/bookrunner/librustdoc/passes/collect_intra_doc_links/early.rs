// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// See GitHub history for details.
use crate::clean;
use crate::html::markdown::markdown_links;
use crate::passes::collect_intra_doc_links::preprocess_link;

use rustc_ast::visit::{self, AssocCtxt, Visitor};
use rustc_ast::{self as ast, ItemKind};
use rustc_ast_lowering::ResolverAstLowering;
use rustc_hir::def::Namespace::TypeNS;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_resolve::Resolver;
use rustc_span::Span;

use std::mem;

struct IntraLinkCrateLoader<'r, 'ra> {
    resolver: &'r mut Resolver<'ra>,
    current_mod: LocalDefId,
    all_traits: Vec<DefId>,
    all_trait_impls: Vec<DefId>,
}

impl IntraLinkCrateLoader<'_, '_> {
    fn load_links_in_attrs(&mut self, attrs: &[ast::Attribute], span: Span) {
        // FIXME: this needs to consider export inlining.
        let attrs = clean::Attributes::from_ast(attrs, None);
        for (parent_module, doc) in attrs.collapsed_doc_value_by_module_level() {
            let module_id = parent_module.unwrap_or(self.current_mod.to_def_id());

            for link in markdown_links(&doc.as_str()) {
                let path_str = if let Some(Ok(x)) = preprocess_link(&link) {
                    x.path_str
                } else {
                    continue;
                };
                let _ = self.resolver.resolve_str_path_error(span, &path_str, TypeNS, module_id);
            }
        }
    }
}

impl Visitor<'_> for IntraLinkCrateLoader<'_, '_> {
    fn visit_item(&mut self, item: &ast::Item) {
        if let ItemKind::Mod(..) = item.kind {
            let old_mod = mem::replace(&mut self.current_mod, self.resolver.local_def_id(item.id));

            self.load_links_in_attrs(&item.attrs, item.span);
            visit::walk_item(self, item);

            self.current_mod = old_mod;
        } else {
            match item.kind {
                ItemKind::Trait(..) => {
                    self.all_traits.push(self.resolver.local_def_id(item.id).to_def_id());
                }
                ItemKind::Impl(box ast::Impl { of_trait: Some(..), .. }) => {
                    self.all_trait_impls.push(self.resolver.local_def_id(item.id).to_def_id());
                }
                _ => {}
            }
            self.load_links_in_attrs(&item.attrs, item.span);
            visit::walk_item(self, item);
        }
    }

    fn visit_assoc_item(&mut self, item: &ast::AssocItem, ctxt: AssocCtxt) {
        self.load_links_in_attrs(&item.attrs, item.span);
        visit::walk_assoc_item(self, item, ctxt)
    }

    fn visit_foreign_item(&mut self, item: &ast::ForeignItem) {
        self.load_links_in_attrs(&item.attrs, item.span);
        visit::walk_foreign_item(self, item)
    }

    fn visit_variant(&mut self, v: &ast::Variant) {
        self.load_links_in_attrs(&v.attrs, v.span);
        visit::walk_variant(self, v)
    }

    fn visit_field_def(&mut self, field: &ast::FieldDef) {
        self.load_links_in_attrs(&field.attrs, field.span);
        visit::walk_field_def(self, field)
    }

    // NOTE: if doc-comments are ever allowed on other nodes (e.g. function parameters),
    // then this will have to implement other visitor methods too.
}
