// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// See GitHub history for details.
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
crate struct ExternalHtml {
    /// Content that will be included inline in the `<head>` section of a
    /// rendered Markdown file or generated documentation
    crate in_header: String,
    /// Content that will be included inline between `<body>` and the content of
    /// a rendered Markdown file or generated documentation
    crate before_content: String,
    /// Content that will be included inline between the content and `</body>` of
    /// a rendered Markdown file or generated documentation
    crate after_content: String,
}

impl ExternalHtml {}
