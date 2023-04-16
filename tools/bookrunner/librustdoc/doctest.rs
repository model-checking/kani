// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use rustc_ast as ast;
use rustc_data_structures::sync::Lrc;
use rustc_errors::{ColorConfig, TerminalUrl};
use rustc_span::edition::Edition;
use rustc_span::source_map::SourceMap;
use rustc_span::symbol::sym;
use rustc_span::FileName;

use std::io::{self};
use std::str;

use crate::html::markdown::LangString;

/// Options that apply to all doctests in a crate or Markdown file (for `rustdoc foo.md`).
#[derive(Clone, Default)]
pub struct GlobalTestOptions {
    /// Whether to disable the default `extern crate my_crate;` when creating doctests.
    pub(crate) no_crate_inject: bool,
    /// Additional crate-level attributes to add to doctests.
    pub(crate) attrs: Vec<String>,
}

/// Transforms a test into code that can be compiled into a Rust binary, and returns the number of
/// lines before the test code begins as well as if the output stream supports colors or not.
pub fn make_test(
    s: &str,
    crate_name: Option<&str>,
    dont_insert_main: bool,
    opts: &GlobalTestOptions,
    edition: Edition,
    test_id: Option<&str>,
) -> (String, usize, bool) {
    let (crate_attrs, everything_else, crates) = partition_source(s);
    let everything_else = everything_else.trim();
    let mut line_offset = 0;
    let mut prog = String::new();
    let mut supports_color = false;

    if opts.attrs.is_empty() {
        // If there aren't any attributes supplied by #![doc(test(attr(...)))], then allow some
        // lints that are commonly triggered in doctests. The crate-level test attributes are
        // commonly used to make tests fail in case they trigger warnings, so having this there in
        // that case may cause some tests to pass when they shouldn't have.
        prog.push_str("#![allow(unused)]\n");
        line_offset += 1;
    }

    // Next, any attributes that came from the crate root via #![doc(test(attr(...)))].
    for attr in &opts.attrs {
        prog.push_str(&format!("#![{attr}]\n"));
        line_offset += 1;
    }

    // Now push any outer attributes from the example, assuming they
    // are intended to be crate attributes.
    prog.push_str(&crate_attrs);
    prog.push_str(&crates);

    // Uses librustc_ast to parse the doctest and find if there's a main fn and the extern
    // crate already is included.
    let result = rustc_driver::catch_fatal_errors(|| {
        rustc_span::create_session_if_not_set_then(edition, |_| {
            use rustc_errors::emitter::{Emitter, EmitterWriter};
            use rustc_errors::Handler;
            use rustc_parse::maybe_new_parser_from_source_str;
            use rustc_parse::parser::ForceCollect;
            use rustc_session::parse::ParseSess;
            use rustc_span::source_map::FilePathMapping;

            let filename = FileName::anon_source_code(s);
            let source = crates + everything_else;

            // Any errors in parsing should also appear when the doctest is compiled for real, so just
            // send all the errors that librustc_ast emits directly into a `Sink` instead of stderr.
            let sm = Lrc::new(SourceMap::new(FilePathMapping::empty()));

            let fallback_bundle =
                rustc_errors::fallback_fluent_bundle(rustc_errors::DEFAULT_LOCALE_RESOURCES, false);
            supports_color = EmitterWriter::stderr(
                ColorConfig::Auto,
                None,
                None,
                fallback_bundle.clone(),
                false,
                false,
                Some(80),
                false,
                false,
                TerminalUrl::No,
            )
            .supports_color();

            let emitter = EmitterWriter::new(
                box io::sink(),
                None,
                None,
                fallback_bundle,
                false,
                false,
                false,
                None,
                false,
                false,
                TerminalUrl::No,
            );

            // FIXME(misdreavus): pass `-Z treat-err-as-bug` to the doctest parser
            let handler = Handler::with_emitter(false, None, box emitter);
            let sess = ParseSess::with_span_handler(handler, sm);

            let mut found_main = false;
            let mut found_extern_crate = crate_name.is_none();
            let mut found_macro = false;

            let mut parser = match maybe_new_parser_from_source_str(&sess, filename, source) {
                Ok(p) => p,
                Err(_errs) => {
                    return (found_main, found_extern_crate, found_macro);
                }
            };

            loop {
                match parser.parse_item(ForceCollect::No) {
                    Ok(Some(item)) => {
                        if !found_main {
                            if let ast::ItemKind::Fn(..) = item.kind {
                                if item.ident.name == sym::main {
                                    found_main = true;
                                }
                            }
                        }

                        if !found_extern_crate {
                            if let ast::ItemKind::ExternCrate(original) = item.kind {
                                // This code will never be reached if `crate_name` is none because
                                // `found_extern_crate` is initialized to `true` if it is none.
                                let crate_name = crate_name.unwrap();

                                match original {
                                    Some(name) => found_extern_crate = name.as_str() == crate_name,
                                    None => found_extern_crate = item.ident.as_str() == crate_name,
                                }
                            }
                        }

                        if !found_macro {
                            if let ast::ItemKind::MacCall(..) = item.kind {
                                found_macro = true;
                            }
                        }

                        if found_main && found_extern_crate {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        e.cancel();
                        break;
                    }
                }

                // The supplied slice is only used for diagnostics,
                // which are swallowed here anyway.
                parser.maybe_consume_incorrect_semicolon(&[]);
            }

            // Reset errors so that they won't be reported as compiler bugs when dropping the
            // handler. Any errors in the tests will be reported when the test file is compiled,
            // Note that we still need to cancel the errors above otherwise `DiagnosticBuilder`
            // will panic on drop.
            sess.span_diagnostic.reset_err_count();

            (found_main, found_extern_crate, found_macro)
        })
    });
    let (already_has_main, already_has_extern_crate, found_macro) = match result {
        Ok(result) => result,
        Err(_) => {
            // If the parser panicked due to a fatal error, pass the test code through unchanged.
            // The error will be reported during compilation.
            return (s.to_owned(), 0, false);
        }
    };

    // If a doctest's `fn main` is being masked by a wrapper macro, the parsing loop above won't
    // see it. In that case, run the old text-based scan to see if they at least have a main
    // function written inside a macro invocation. See
    // https://github.com/rust-lang/rust/issues/56898
    let already_has_main = if found_macro && !already_has_main {
        s.lines()
            .map(|line| {
                let comment = line.find("//");
                if let Some(comment_begins) = comment { &line[0..comment_begins] } else { line }
            })
            .any(|code| code.contains("fn main"))
    } else {
        already_has_main
    };

    // Don't inject `extern crate std` because it's already injected by the
    // compiler.
    if !already_has_extern_crate && !opts.no_crate_inject && crate_name != Some("std") {
        if let Some(crate_name) = crate_name {
            // Don't inject `extern crate` if the crate is never used.
            // NOTE: this is terribly inaccurate because it doesn't actually
            // parse the source, but only has false positives, not false
            // negatives.
            if s.contains(crate_name) {
                prog.push_str(&format!("extern crate r#{crate_name};\n"));
                line_offset += 1;
            }
        }
    }

    // FIXME: This code cannot yet handle no_std test cases yet
    if dont_insert_main || already_has_main || prog.contains("![no_std]") {
        prog.push_str(everything_else);
    } else {
        let returns_result = everything_else.trim_end().ends_with("(())");
        // Give each doctest main function a unique name.
        // This is for example needed for the tooling around `-Z instrument-coverage`.
        let inner_fn_name = if let Some(test_id) = test_id {
            format!("_doctest_main_{test_id}")
        } else {
            "_inner".into()
        };
        let inner_attr = if test_id.is_some() { "#[allow(non_snake_case)] " } else { "" };
        let (main_pre, main_post) = if returns_result {
            (
                format!(
                    "fn main() {{ {inner_attr}fn {inner_fn_name}() -> Result<(), impl core::fmt::Debug> {{\n"
                ),
                format!("\n}} {inner_fn_name}().unwrap() }}"),
            )
        } else if test_id.is_some() {
            (
                format!("fn main() {{ {inner_attr}fn {inner_fn_name}() {{\n"),
                format!("\n}} {inner_fn_name}() }}"),
            )
        } else {
            ("fn main() {\n".into(), "\n}".into())
        };
        // Note on newlines: We insert a line/newline *before*, and *after*
        // the doctest and adjust the `line_offset` accordingly.
        // In the case of `-Z instrument-coverage`, this means that the generated
        // inner `main` function spans from the doctest opening codeblock to the
        // closing one. For example
        // /// ``` <- start of the inner main
        // /// <- code under doctest
        // /// ``` <- end of the inner main
        line_offset += 1;

        prog.extend([&main_pre, everything_else, &main_post].iter().cloned());
    }

    debug!("final doctest:\n{}", prog);

    (prog, line_offset, supports_color)
}

// FIXME(aburka): use a real parser to deal with multiline attributes
fn partition_source(s: &str) -> (String, String, String) {
    #[derive(Copy, Clone, PartialEq)]
    enum PartitionState {
        Attrs,
        Crates,
        Other,
    }
    let mut state = PartitionState::Attrs;
    let mut before = String::new();
    let mut crates = String::new();
    let mut after = String::new();

    for line in s.lines() {
        let trimline = line.trim();

        // FIXME(misdreavus): if a doc comment is placed on an extern crate statement, it will be
        // shunted into "everything else"
        match state {
            PartitionState::Attrs => {
                state = if trimline.starts_with("#![")
                    || trimline.chars().all(|c| c.is_whitespace())
                    || (trimline.starts_with("//") && !trimline.starts_with("///"))
                {
                    PartitionState::Attrs
                } else if trimline.starts_with("extern crate")
                    || trimline.starts_with("#[macro_use] extern crate")
                {
                    PartitionState::Crates
                } else {
                    PartitionState::Other
                };
            }
            PartitionState::Crates => {
                state = if trimline.starts_with("extern crate")
                    || trimline.starts_with("#[macro_use] extern crate")
                    || trimline.chars().all(|c| c.is_whitespace())
                    || (trimline.starts_with("//") && !trimline.starts_with("///"))
                {
                    PartitionState::Crates
                } else {
                    PartitionState::Other
                };
            }
            PartitionState::Other => {}
        }

        match state {
            PartitionState::Attrs => {
                before.push_str(line);
                before.push('\n');
            }
            PartitionState::Crates => {
                crates.push_str(line);
                crates.push('\n');
            }
            PartitionState::Other => {
                after.push_str(line);
                after.push('\n');
            }
        }
    }

    debug!("before:\n{}", before);
    debug!("crates:\n{}", crates);
    debug!("after:\n{}", after);

    (before, after, crates)
}

pub trait Tester {
    fn add_test(&mut self, test: String, config: LangString, line: usize);
    fn get_line(&self) -> usize {
        0
    }
    fn register_header(&mut self, _name: &str, _level: u32) {}
}
