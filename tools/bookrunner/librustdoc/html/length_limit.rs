// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! See [`HtmlWithLimit`].

/// A buffer that allows generating HTML with a length limit.
///
/// This buffer ensures that:
///
/// * all tags are closed,
/// * tags are closed in the reverse order of when they were opened (i.e., the correct HTML order),
/// * no tags are left empty (e.g., `<em></em>`) due to the length limit being reached,
/// * all text is escaped.
#[derive(Debug)]
pub(super) struct HtmlWithLimit {
    buf: String,
    len: usize,
    limit: usize,
    /// A list of tags that have been requested to be opened via [`Self::open_tag()`]
    /// but have not actually been pushed to `buf` yet. This ensures that tags are not
    /// left empty (e.g., `<em></em>`) due to the length limit being reached.
    queued_tags: Vec<&'static str>,
    /// A list of all tags that have been opened but not yet closed.
    unclosed_tags: Vec<&'static str>,
}

#[cfg(test)]
mod tests;
