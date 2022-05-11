// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! Contains information about "passes", used to modify crate information during the documentation
//! process.

use rustc_middle::ty::TyCtxt;
use rustc_span::{InnerSpan, Span, DUMMY_SP};
use std::ops::Range;

use crate::clean::{self, DocFragmentKind};

mod unindent_comments;

crate mod collect_intra_doc_links;

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

/// Attempts to match a range of bytes from parsed markdown to a `Span` in the source code.
///
/// This method will return `None` if we cannot construct a span from the source map or if the
/// attributes are not all sugared doc comments. It's difficult to calculate the correct span in
/// that case due to escaping and other source features.
crate fn source_span_for_markdown_range(
    tcx: TyCtxt<'_>,
    markdown: &str,
    md_range: &Range<usize>,
    attrs: &clean::Attributes,
) -> Option<Span> {
    let is_all_sugared_doc =
        attrs.doc_strings.iter().all(|frag| frag.kind == DocFragmentKind::SugaredDoc);

    if !is_all_sugared_doc {
        return None;
    }

    let snippet = tcx.sess.source_map().span_to_snippet(span_of_attrs(attrs)?).ok()?;

    let starting_line = markdown[..md_range.start].matches('\n').count();
    let ending_line = starting_line + markdown[md_range.start..md_range.end].matches('\n').count();

    // We use `split_terminator('\n')` instead of `lines()` when counting bytes so that we treat
    // CRLF and LF line endings the same way.
    let mut src_lines = snippet.split_terminator('\n');
    let md_lines = markdown.split_terminator('\n');

    // The number of bytes from the source span to the markdown span that are not part
    // of the markdown, like comment markers.
    let mut start_bytes = 0;
    let mut end_bytes = 0;

    'outer: for (line_no, md_line) in md_lines.enumerate() {
        loop {
            let source_line = src_lines.next().expect("could not find markdown in source");
            match source_line.find(md_line) {
                Some(offset) => {
                    if line_no == starting_line {
                        start_bytes += offset;

                        if starting_line == ending_line {
                            break 'outer;
                        }
                    } else if line_no == ending_line {
                        end_bytes += offset;
                        break 'outer;
                    } else if line_no < starting_line {
                        start_bytes += source_line.len() - md_line.len();
                    } else {
                        end_bytes += source_line.len() - md_line.len();
                    }
                    break;
                }
                None => {
                    // Since this is a source line that doesn't include a markdown line,
                    // we have to count the newline that we split from earlier.
                    if line_no <= starting_line {
                        start_bytes += source_line.len() + 1;
                    } else {
                        end_bytes += source_line.len() + 1;
                    }
                }
            }
        }
    }

    Some(span_of_attrs(attrs)?.from_inner(InnerSpan::new(
        md_range.start + start_bytes,
        md_range.end + start_bytes + end_bytes,
    )))
}
