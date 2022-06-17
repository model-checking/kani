// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ExternalHtml {
    /// Content that will be included inline in the `<head>` section of a
    /// rendered Markdown file or generated documentation
    pub(crate) in_header: String,
    /// Content that will be included inline between `<body>` and the content of
    /// a rendered Markdown file or generated documentation
    pub(crate) before_content: String,
    /// Content that will be included inline between the content and `</body>` of
    /// a rendered Markdown file or generated documentation
    pub(crate) after_content: String,
}

impl ExternalHtml {}
