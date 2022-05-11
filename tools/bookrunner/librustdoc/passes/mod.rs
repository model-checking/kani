// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! Contains information about "passes", used to modify crate information during the documentation
//! process.

use rustc_span::{Span, DUMMY_SP};

use crate::clean::{self};

mod check_doc_test_visibility;

/// Returns a span encompassing all the given attributes.
crate fn span_of_attrs(attrs: &clean::Attributes) -> Option<Span> {
    if attrs.doc_strings.is_empty() {
        return None;
    }
    let start = attrs.doc_strings[0].span;
    if start == DUMMY_SP {
        return None;
    }
    let end = attrs.doc_strings.last().expect("no doc strings provided").span;
    Some(start.to(end))
}
