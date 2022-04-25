// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use std::convert::TryFrom;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use rustc_data_structures::fx::FxHashMap;
use rustc_session::config::{ErrorOutputType, Externs};
use rustc_session::lint::Level;
use rustc_session::search_paths::SearchPath;
use rustc_span::edition::Edition;
use rustc_target::spec::TargetTriple;

use crate::externalfiles::ExternalHtml;
use crate::html::markdown::IdMap;
use crate::html::render::StylePath;
use crate::scrape_examples::AllCallLocations;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
crate enum OutputFormat {
    Json,
    Html,
}

impl Default for OutputFormat {
    fn default() -> OutputFormat {
        OutputFormat::Html
    }
}

impl OutputFormat {}

impl TryFrom<&str> for OutputFormat {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "json" => Ok(OutputFormat::Json),
            "html" => Ok(OutputFormat::Html),
            _ => Err(format!("unknown output format `{}`", value)),
        }
    }
}

/// Configuration options for rustdoc.
#[derive(Clone)]
crate struct Options {
    // Basic options / Options passed directly to rustc
    /// The crate root or Markdown file to load.
    crate input: PathBuf,
    /// The name of the crate being documented.
    crate crate_name: Option<String>,
    /// Whether or not this is a proc-macro crate
    crate proc_macro_crate: bool,
    /// How to format errors and warnings.
    crate error_format: ErrorOutputType,
    /// Library search paths to hand to the compiler.
    crate libs: Vec<SearchPath>,
    /// Library search paths strings to hand to the compiler.
    crate lib_strs: Vec<String>,
    /// The list of external crates to link against.
    crate externs: Externs,
    /// The list of external crates strings to link against.
    crate extern_strs: Vec<String>,
    /// List of `cfg` flags to hand to the compiler. Always includes `rustdoc`.
    crate cfgs: Vec<String>,
    /// Codegen options strings to hand to the compiler.
    crate codegen_options_strs: Vec<String>,
    /// Debugging (`-Z`) options strings to pass to the compiler.
    crate debugging_opts_strs: Vec<String>,
    /// The target used to compile the crate against.
    crate target: TargetTriple,
    /// Edition used when reading the crate. Defaults to "2015". Also used by default when
    /// compiling doctests from the crate.
    crate edition: Edition,
    /// The path to the sysroot. Used during the compilation process.
    crate maybe_sysroot: Option<PathBuf>,
    /// Lint information passed over the command-line.
    crate lint_opts: Vec<(String, Level)>,
    /// Whether to ask rustc to describe the lints it knows.
    crate describe_lints: bool,
    /// What level to cap lints at.
    crate lint_cap: Option<Level>,

    // Options specific to running doctests
    /// Whether we should run doctests instead of generating docs.
    crate should_test: bool,
    /// List of arguments to pass to the test harness, if running tests.
    crate test_args: Vec<String>,
    /// The working directory in which to run tests.
    crate test_run_directory: Option<PathBuf>,
    /// Optional path to persist the doctest executables to, defaults to a
    /// temporary directory if not set.
    crate persist_doctests: Option<PathBuf>,
    /// Runtool to run doctests with
    crate runtool: Option<String>,
    /// Arguments to pass to the runtool
    crate runtool_args: Vec<String>,
    /// Whether to allow ignoring doctests on a per-target basis
    /// For example, using ignore-foo to ignore running the doctest on any target that
    /// contains "foo" as a substring
    crate enable_per_target_ignores: bool,
    /// Do not run doctests, compile them if should_test is active.
    crate no_run: bool,

    /// The path to a rustc-like binary to build tests with. If not set, we
    /// default to loading from `$sysroot/bin/rustc`.
    crate test_builder: Option<PathBuf>,

    // Options that affect the documentation process
    /// Whether to run the `calculate-doc-coverage` pass, which counts the number of public items
    /// with and without documentation.
    crate show_coverage: bool,

    // Options that alter generated documentation pages
    /// Crate version to note on the sidebar of generated docs.
    crate crate_version: Option<String>,
    /// Collected options specific to outputting final pages.
    crate render_options: RenderOptions,
    /// If this option is set to `true`, rustdoc will only run checks and not generate
    /// documentation.
    crate run_check: bool,
    /// Whether doctests should emit unused externs
    crate json_unused_externs: bool,
    /// Whether to skip capturing stdout and stderr of tests.
    crate nocapture: bool,
}

impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct FmtExterns<'a>(&'a Externs);

        impl<'a> fmt::Debug for FmtExterns<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_map().entries(self.0.iter()).finish()
            }
        }

        f.debug_struct("Options")
            .field("input", &self.input)
            .field("crate_name", &self.crate_name)
            .field("proc_macro_crate", &self.proc_macro_crate)
            .field("error_format", &self.error_format)
            .field("libs", &self.libs)
            .field("externs", &FmtExterns(&self.externs))
            .field("cfgs", &self.cfgs)
            .field("codegen_options", &"...")
            .field("debugging_options", &"...")
            .field("target", &self.target)
            .field("edition", &self.edition)
            .field("maybe_sysroot", &self.maybe_sysroot)
            .field("lint_opts", &self.lint_opts)
            .field("describe_lints", &self.describe_lints)
            .field("lint_cap", &self.lint_cap)
            .field("should_test", &self.should_test)
            .field("test_args", &self.test_args)
            .field("test_run_directory", &self.test_run_directory)
            .field("persist_doctests", &self.persist_doctests)
            .field("show_coverage", &self.show_coverage)
            .field("crate_version", &self.crate_version)
            .field("render_options", &self.render_options)
            .field("runtool", &self.runtool)
            .field("runtool_args", &self.runtool_args)
            .field("enable-per-target-ignores", &self.enable_per_target_ignores)
            .field("run_check", &self.run_check)
            .field("no_run", &self.no_run)
            .field("nocapture", &self.nocapture)
            .finish()
    }
}

/// Configuration options for the HTML page-creation process.
#[derive(Clone, Debug)]
crate struct RenderOptions {
    /// Output directory to generate docs into. Defaults to `doc`.
    crate output: PathBuf,
    /// External files to insert into generated pages.
    crate external_html: ExternalHtml,
    /// A pre-populated `IdMap` with the default headings and any headings added by Markdown files
    /// processed by `external_html`.
    crate id_map: IdMap,
    /// If present, playground URL to use in the "Run" button added to code samples.
    ///
    /// Be aware: This option can come both from the CLI and from crate attributes!
    crate playground_url: Option<String>,
    /// Whether to sort modules alphabetically on a module page instead of using declaration order.
    /// `true` by default.
    //
    // FIXME(misdreavus): the flag name is `--sort-modules-by-appearance` but the meaning is
    // inverted once read.
    crate sort_modules_alphabetically: bool,
    /// List of themes to extend the docs with. Original argument name is included to assist in
    /// displaying errors if it fails a theme check.
    crate themes: Vec<StylePath>,
    /// If present, CSS file that contains rules to add to the default CSS.
    crate extension_css: Option<PathBuf>,
    /// A map of the default settings (values are as for DOM storage API). Keys should lack the
    /// `rustdoc-` prefix.
    crate default_settings: FxHashMap<String, String>,
    /// If present, suffix added to CSS/JavaScript files when referencing them in generated pages.
    crate resource_suffix: String,
    /// Whether to run the static CSS/JavaScript through a minifier when outputting them. `true` by
    /// default.
    //
    // FIXME(misdreavus): the flag name is `--disable-minification` but the meaning is inverted
    // once read.
    crate enable_minification: bool,
    /// Whether to create an index page in the root of the output directory. If this is true but
    /// `enable_index_page` is None, generate a static listing of crates instead.
    crate enable_index_page: bool,
    /// A file to use as the index page at the root of the output directory. Overrides
    /// `enable_index_page` to be true if set.
    crate index_page: Option<PathBuf>,
    /// An optional path to use as the location of static files. If not set, uses combinations of
    /// `../` to reach the documentation root.
    crate static_root_path: Option<String>,

    // Options specific to reading standalone Markdown files
    /// Whether to generate a table of contents on the output file when reading a standalone
    /// Markdown file.
    crate markdown_no_toc: bool,
    /// Additional CSS files to link in pages generated from standalone Markdown files.
    crate markdown_css: Vec<String>,
    /// If present, playground URL to use in the "Run" button added to code samples generated from
    /// standalone Markdown files. If not present, `playground_url` is used.
    crate markdown_playground_url: Option<String>,
    /// Document items that have lower than `pub` visibility.
    crate document_private: bool,
    /// Document items that have `doc(hidden)`.
    crate document_hidden: bool,
    /// If `true`, generate a JSON file in the crate folder instead of HTML redirection files.
    crate generate_redirect_map: bool,
    /// Show the memory layout of types in the docs.
    crate show_type_layout: bool,
    crate unstable_features: rustc_feature::UnstableFeatures,
    crate emit: Vec<EmitType>,
    /// If `true`, HTML source pages will generate links for items to their definition.
    crate generate_link_to_definition: bool,
    /// Set of function-call locations to include as examples
    crate call_locations: AllCallLocations,
    /// If `true`, Context::init will not emit shared files.
    crate no_emit_shared: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
crate enum EmitType {
    Unversioned,
    Toolchain,
    InvocationSpecific,
}

impl FromStr for EmitType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use EmitType::*;
        match s {
            "unversioned-shared-resources" => Ok(Unversioned),
            "toolchain-shared-resources" => Ok(Toolchain),
            "invocation-specific" => Ok(InvocationSpecific),
            _ => Err(()),
        }
    }
}

impl RenderOptions {
    crate fn should_emit_crate(&self) -> bool {
        self.emit.is_empty() || self.emit.contains(&EmitType::InvocationSpecific)
    }
}

impl Options {}
