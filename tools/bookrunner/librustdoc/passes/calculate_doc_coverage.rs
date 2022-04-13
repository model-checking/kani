// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// See GitHub history for details.
//! Calculates information used for the --show-coverage flag.
use crate::clean;
use crate::core::DocContext;
use crate::html::markdown::{find_testable_code, ErrorCodes};
use crate::passes::check_doc_test_visibility::{should_have_doc_example, Tests};
use crate::visit::DocVisitor;
use rustc_hir as hir;
use rustc_lint::builtin::MISSING_DOCS;
use rustc_middle::lint::LintLevelSource;
use rustc_middle::ty::DefIdTree;
use rustc_session::lint;
use rustc_span::FileName;
use serde::Serialize;

use std::collections::BTreeMap;
use std::ops;

#[derive(Default, Copy, Clone, Serialize, Debug)]
struct ItemCount {
    total: u64,
    with_docs: u64,
    total_examples: u64,
    with_examples: u64,
}

impl ItemCount {
    fn count_item(
        &mut self,
        has_docs: bool,
        has_doc_example: bool,
        should_have_doc_examples: bool,
        should_have_docs: bool,
    ) {
        if has_docs || should_have_docs {
            self.total += 1;
        }

        if has_docs {
            self.with_docs += 1;
        }
        if should_have_doc_examples || has_doc_example {
            self.total_examples += 1;
        }
        if has_doc_example {
            self.with_examples += 1;
        }
    }
}

impl ops::Sub for ItemCount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        ItemCount {
            total: self.total - rhs.total,
            with_docs: self.with_docs - rhs.with_docs,
            total_examples: self.total_examples - rhs.total_examples,
            with_examples: self.with_examples - rhs.with_examples,
        }
    }
}

impl ops::AddAssign for ItemCount {
    fn add_assign(&mut self, rhs: Self) {
        self.total += rhs.total;
        self.with_docs += rhs.with_docs;
        self.total_examples += rhs.total_examples;
        self.with_examples += rhs.with_examples;
    }
}

struct CoverageCalculator<'a, 'b> {
    items: BTreeMap<FileName, ItemCount>,
    ctx: &'a mut DocContext<'b>,
}

impl<'a, 'b> CoverageCalculator<'a, 'b> {}

impl<'a, 'b> DocVisitor for CoverageCalculator<'a, 'b> {
    fn visit_item(&mut self, i: &clean::Item) {
        if !i.def_id.is_local() {
            // non-local items are skipped because they can be out of the users control,
            // especially in the case of trait impls, which rustdoc eagerly inlines
            return;
        }

        match *i.kind {
            clean::StrippedItem(..) => {
                // don't count items in stripped modules
                return;
            }
            // docs on `use` and `extern crate` statements are not displayed, so they're not
            // worth counting
            clean::ImportItem(..) | clean::ExternCrateItem { .. } => {}
            // Don't count trait impls, the missing-docs lint doesn't so we shouldn't either.
            // Inherent impls *can* be documented, and those docs show up, but in most cases it
            // doesn't make sense, as all methods on a type are in one single impl block
            clean::ImplItem(_) => {}
            _ => {
                let has_docs = !i.attrs.doc_strings.is_empty();
                let mut tests = Tests { found_tests: 0 };

                find_testable_code(
                    &i.attrs.collapsed_doc_value().unwrap_or_default(),
                    &mut tests,
                    ErrorCodes::No,
                    false,
                    None,
                );

                let filename = i.span(self.ctx.tcx).filename(self.ctx.sess());
                let has_doc_example = tests.found_tests != 0;
                // The `expect_def_id()` should be okay because `local_def_id_to_hir_id`
                // would presumably panic if a fake `DefIndex` were passed.
                let hir_id = self
                    .ctx
                    .tcx
                    .hir()
                    .local_def_id_to_hir_id(i.def_id.expect_def_id().expect_local());
                let (level, source) = self.ctx.tcx.lint_level_at_node(MISSING_DOCS, hir_id);

                // In case we have:
                //
                // ```
                // enum Foo { Bar(u32) }
                // // or:
                // struct Bar(u32);
                // ```
                //
                // there is no need to require documentation on the fields of tuple variants and
                // tuple structs.
                let should_be_ignored = i
                    .def_id
                    .as_def_id()
                    .and_then(|def_id| self.ctx.tcx.parent(def_id))
                    .and_then(|def_id| self.ctx.tcx.hir().get_if_local(def_id))
                    .map(|node| {
                        matches!(
                            node,
                            hir::Node::Variant(hir::Variant {
                                data: hir::VariantData::Tuple(_, _),
                                ..
                            }) | hir::Node::Item(hir::Item {
                                kind: hir::ItemKind::Struct(hir::VariantData::Tuple(_, _), _),
                                ..
                            })
                        )
                    })
                    .unwrap_or(false);

                // `missing_docs` is allow-by-default, so don't treat this as ignoring the item
                // unless the user had an explicit `allow`.
                //
                let should_have_docs = !should_be_ignored
                    && (level != lint::Level::Allow || matches!(source, LintLevelSource::Default));

                debug!("counting {:?} {:?} in {:?}", i.type_(), i.name, filename);
                self.items.entry(filename).or_default().count_item(
                    has_docs,
                    has_doc_example,
                    should_have_doc_example(self.ctx, &i),
                    should_have_docs,
                );
            }
        }

        self.visit_item_recur(i)
    }
}
