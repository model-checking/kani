// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use std::path::PathBuf;

use rustc_data_structures::fx::FxHashMap;

use crate::error::Error;
use crate::externalfiles::ExternalHtml;

#[allow(dead_code)]
#[derive(Clone)]
crate struct Layout {
    crate logo: String,
    crate favicon: String,
    crate external_html: ExternalHtml,
    crate default_settings: FxHashMap<String, String>,
    crate krate: String,
    /// The given user css file which allow to customize the generated
    /// documentation theme.
    crate css_file_extension: Option<PathBuf>,
    /// If true, then scrape-examples.js will be included in the output HTML file
    crate scrape_examples_extension: bool,
}

#[allow(dead_code)]
crate struct Page<'a> {
    crate title: &'a str,
    crate css_class: &'a str,
    crate root_path: &'a str,
    crate static_root_path: Option<&'a str>,
    crate description: &'a str,
    crate keywords: &'a str,
    crate resource_suffix: &'a str,
    crate extra_scripts: &'a [&'a str],
    crate static_extra_scripts: &'a [&'a str],
}

impl<'a> Page<'a> {
    crate fn get_static_root_path(&self) -> &str {
        self.static_root_path.unwrap_or(self.root_path)
    }
}

#[allow(dead_code)]
struct PageLayout<'a> {
    static_root_path: &'a str,
    page: &'a Page<'a>,
    layout: &'a Layout,
    themes: Vec<String>,
    sidebar: String,
    content: String,
    krate_with_trailing_slash: String,
    crate rustdoc_version: &'a str,
}
