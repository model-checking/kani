// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Utilities to extract examples from Rust books, run them through RMC, and
//! display their results.

extern crate rustc_span;

use crate::{
    dashboard,
    litani::{Litani, LitaniRun, LitaniPipeline},
    util::{self, FailStep, TestProps},
};
use inflector::cases::{snakecase::to_snake_case, titlecase::to_title_case};
use pulldown_cmark::{Event, Parser, Tag};
use rustc_span::edition::Edition;
use rustdoc::{
    doctest::Tester,
    html::markdown::{ErrorCodes, Ignore, LangString},
};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt::Write,
    fs,
    io::BufReader,
    iter::FromIterator,
    path::{Path, PathBuf},
};
use serde_json;
use walkdir::WalkDir;

/// Parses the chapter/section hierarchy in the markdown file specified by
/// `summary_path` and returns a mapping from markdown files containing rust
/// code to corresponding directories where the extracted rust code should
/// reside.
fn parse_hierarchy(
    book_name: &str,
    summary_path: PathBuf,
    summary_start: &str,
    mut map: HashMap<PathBuf, PathBuf>,
) -> HashMap<PathBuf, PathBuf> {
    let summary_dir = summary_path.parent().unwrap().to_path_buf();
    let summary = fs::read_to_string(summary_path).unwrap();
    assert!(
        summary.starts_with(summary_start),
        "Error: The start of {} summary file changed.",
        book_name
    );
    // Skip the `start` of the summary.
    let n = Parser::new(summary_start).count();
    let parser = Parser::new(&summary).skip(n);
    // Set `book_name` as the root of the hierarchical path.
    let mut hierarchy: PathBuf = ["src", "test", "dashboard", "books", book_name].iter().collect();
    let mut prev_event_is_text_or_code = false;
    for event in parser {
        match event {
            Event::End(Tag::Item) => {
                // Pop the current chapter/section from the hierarchy once
                // we are done processing it and its subsections.
                hierarchy.pop();
                prev_event_is_text_or_code = false;
            }
            Event::End(Tag::Link(_, path, _)) => {
                // At the start of the link tag, the hierarchy does not yet
                // contain the title of the current chapter/section. So, we wait
                // for the end of the link tag before adding the path and
                // hierarchy of the current chapter/section to the map.
                let mut full_path = summary_dir.clone();
                full_path.extend(path.split('/'));
                map.insert(full_path, hierarchy.clone());
                prev_event_is_text_or_code = false;
            }
            Event::Text(text) | Event::Code(text) => {
                // Remove characters that are problematic to the file system or
                // terminal.
                let text = text.replace(&['/', '(', ')', '\''][..], "_");
                // Does the chapter/section title contain normal text and inline
                // code?
                if prev_event_is_text_or_code {
                    // If so, we combine them into one hierarchy level.
                    let prev_text = hierarchy.file_name().unwrap().to_str().unwrap().to_string();
                    hierarchy.pop();
                    hierarchy.push(format!("{}{}", prev_text, text));
                } else {
                    // If not, add the current title to the hierarchy.
                    hierarchy.push(text.to_string());
                }
                prev_event_is_text_or_code = true;
            }
            _ => (),
        }
    }
    map
}

/// Parses [The Rust Reference](https://doc.rust-lang.org/nightly/reference)
/// book.
fn parse_reference_hierarchy() -> HashMap<PathBuf, PathBuf> {
    parse_hierarchy(
        "The Rust Reference",
        ["src", "doc", "reference", "src", "SUMMARY.md"].iter().collect(),
        "# The Rust Reference\n\n[Introduction](introduction.md)",
        HashMap::from_iter([(
            ["src", "doc", "reference", "src", "introduction.md"].iter().collect(),
            ["src", "test", "dashboard", "books", "The Rust Reference", "Introduction"]
                .iter()
                .collect(),
        )]),
    )
}

/// Parses [The Rustonomicon](https://doc.rust-lang.org/nightly/nomicon) book.
fn parse_nomicon_hierarchy() -> HashMap<PathBuf, PathBuf> {
    parse_hierarchy(
        "The Rustonomicon",
        ["src", "doc", "nomicon", "src", "SUMMARY.md"].iter().collect(),
        "# Summary\n\n[Introduction](intro.md)",
        HashMap::from_iter([(
            ["src", "doc", "nomicon", "src", "intro.md"].iter().collect(),
            ["src", "test", "dashboard", "books", "The Rustonomicon", "Introduction"]
                .iter()
                .collect(),
        )]),
    )
}

/// Parses the
/// [Rust by Example](https://doc.rust-lang.org/nightly/rust-by-example) book.
fn parse_rust_by_example_hierarchy() -> HashMap<PathBuf, PathBuf> {
    parse_hierarchy(
        "Rust by Example",
        ["src", "doc", "rust-by-example", "src", "SUMMARY.md"].iter().collect(),
        "# Summary\n\n[Introduction](index.md)",
        HashMap::from_iter([(
            ["src", "doc", "rust-by-example", "src", "index.md"].iter().collect(),
            ["src", "test", "dashboard", "books", "Rust by Example", "Introduction"]
                .iter()
                .collect(),
        )]),
    )
}

/// Parses [The Unstable Book](https://doc.rust-lang.org/nightly/unstable-book).
/// Unlike the other books, this one does not have a `SUMMARY.md` file (i.e., a
/// table of contents). So we parse it manually and make a "best effort" to make
/// it look like the online version.
fn parse_unstable_book_hierarchy() -> HashMap<PathBuf, PathBuf> {
    // Keeps track of directory we are currently processing, starting from root
    // of the book.
    let mut src_prefix: PathBuf = ["src", "doc", "unstable-book", "src"].iter().collect();
    // Corresponding directory where the examples extracted from the book should
    // reside.
    let mut dest_prefix: PathBuf =
        ["src", "test", "dashboard", "books", "The Unstable Book"].iter().collect();
    let mut map = HashMap::new();
    for entry in WalkDir::new(&src_prefix) {
        let entry = entry.unwrap().into_path();
        // `WalkDir` returns entries in a depth-first fashion. Once we are done
        // processing a directory, it will jump to a different child entry of a
        // predecessor. To copy examples to the correct location, we need to
        // know how far back we jumped and update `dest_prefix` accordingly.
        while !entry.starts_with(&src_prefix) {
            src_prefix.pop();
            dest_prefix.pop();
        }
        if entry.is_dir() {
            src_prefix.push(entry.file_name().unwrap());
            // Follow the book's title case format for directories.
            dest_prefix.push(to_title_case(entry.file_name().unwrap().to_str().unwrap()));
        } else {
            // Only process markdown files.
            if entry.extension() == Some(OsStr::new("md")) {
                let entry_stem = entry.file_stem().unwrap().to_str().unwrap();
                // If a file has the stem name as a sibling directory...
                if src_prefix.join(entry.file_stem().unwrap()).exists() {
                    // Its extracted examples should reside under that
                    // directory.
                    map.insert(entry.clone(), dest_prefix.join(to_title_case(entry_stem)));
                } else {
                    // Otherwise, follow the book's snake case format for files.
                    map.insert(entry.clone(), dest_prefix.join(to_snake_case(entry_stem)));
                }
            }
        }
    }
    map
}

/// This data structure contains the code and configs of an example in the Rust books.
struct Example {
    /// The example code extracted from a codeblock.
    code: String,
    // Line number of the code block.
    line: usize,
    // Configurations in the header of the codeblock.
    config: rustdoc::html::markdown::LangString,
}

/// Data structure representing a list of examples. Mainly for implementing the
/// [`Tester`] trait.
struct Examples(Vec<Example>);

impl Tester for Examples {
    fn add_test(&mut self, test: String, config: LangString, line: usize) {
        if config.ignore != Ignore::All {
            self.0.push(Example { code: test, line, config })
        }
    }
}

/// Applies the diff corresponding to `example` with parent `path` (if it exists).
fn apply_diff(path: &Path, example: &mut Example, config_paths: &mut HashSet<PathBuf>) {
    let config_dir: PathBuf = ["src", "tools", "dashboard", "configs"].iter().collect();
    let test_dir: PathBuf = ["src", "test", "dashboard"].iter().collect();
    // `path` has the following form:
    // `src/test/dashboard/books/<hierarchy>
    // If `example` has a custom diff file, the path to the diff file will have
    // the following form:
    // `src/tools/dashboard/configs/books/<hierarchy>/<example.line>.diff`
    // where <hierarchy> is the same for both paths.
    let mut diff_path = config_dir.join(path.strip_prefix(&test_dir).unwrap());
    diff_path.extend_one(format!("{}.diff", example.line));
    if diff_path.exists() {
        config_paths.remove(&diff_path);
        let mut code_lines: Vec<_> = example.code.lines().collect();
        let diff = fs::read_to_string(diff_path).unwrap();
        for line in diff.lines() {
            // `*.diff` files have a simple format:
            // `- <line-num>` for removing lines.
            // `+ <line-num> <code>` for inserting lines.
            // Notice that for a series of `+` and `-`, the developer must keep
            // track of the changing line numbers.
            let mut split = line.splitn(3, ' ');
            let symbol = split.next().unwrap();
            let line = split.next().unwrap().parse::<usize>().unwrap() - 1;
            if symbol == "+" {
                let diff = split.next().unwrap();
                code_lines.insert(line, diff);
            } else {
                code_lines.remove(line);
            }
        }
        example.code = code_lines.join("\n");
    }
}

/// Prepends example properties in `example.config` to the code in `example.code`.
fn prepend_props(path: &Path, example: &mut Example, config_paths: &mut HashSet<PathBuf>) {
    let config_dir: PathBuf = ["src", "tools", "dashboard", "configs"].iter().collect();
    let test_dir: PathBuf = ["src", "test", "dashboard"].iter().collect();
    // `path` has the following form:
    // `src/test/dashboard/books/<hierarchy>
    // If `example` has a custom props file, the path to the props file will
    // have the following form:
    // `src/tools/dashboard/configs/books/<hierarchy>/<example.line>.props`
    // where <hierarchy> is the same for both paths.
    let mut props_path = config_dir.join(path.strip_prefix(&test_dir).unwrap());
    props_path.extend_one(format!("{}.props", example.line));
    let mut props = if props_path.exists() {
        config_paths.remove(&props_path);
        util::parse_test_header(&props_path)
    } else {
        TestProps::new(path.to_path_buf(), None, Vec::new(), Vec::new())
    };
    if example.config.edition != Some(Edition::Edition2015) {
        props.rustc_args.push(String::from("--edition"));
        props.rustc_args.push(String::from("2018"));
    }
    if props.fail_step.is_none() {
        if example.config.compile_fail {
            // Most examples with `compile_fail` annotation fail because of
            // check errors. This heuristic can be overridden by manually
            //specifying the fail step in the corresponding config file.
            props.fail_step = Some(FailStep::Check);
        } else if example.config.should_panic {
            // RMC should catch run-time errors.
            props.fail_step = Some(FailStep::Verification);
        }
    }
    example.code = format!("{}{}", props, example.code);
}

/// Make the main function of a test public so it can be verified by rmc.
fn pub_main(code: String) -> String {
    code.replace("fn main", "pub fn main")
}

/// Extracts examples from the markdown file specified by `par_from`,
/// pre-processes those examples, and saves them in the directory specified by
/// `par_to`.
fn extract(par_from: &Path, par_to: &Path, config_paths: &mut HashSet<PathBuf>) {
    let code = fs::read_to_string(&par_from).unwrap();
    let mut examples = Examples(Vec::new());
    rustdoc::html::markdown::find_testable_code(&code, &mut examples, ErrorCodes::No, false, None);
    for mut example in examples.0 {
        apply_diff(par_to, &mut example, config_paths);
        example.code = pub_main(
            rustdoc::doctest::make_test(
                &example.code,
                None,
                false,
                &Default::default(),
                example.config.edition.unwrap_or(Edition::Edition2018),
                None,
            )
            .0,
        );
        prepend_props(par_to, &mut example, config_paths);
        let rs_path = par_to.join(format!("{}.rs", example.line));
        fs::create_dir_all(rs_path.parent().unwrap()).unwrap();
        fs::write(rs_path, example.code).unwrap();
    }
}

/// Extracts examples from the markdown files specified by each key in the given
/// `map`, pre-processes those examples, and saves them in the directory
/// specified by the corresponding value.
fn extract_examples(par_map: HashMap<PathBuf, PathBuf>) {
    let mut config_paths = get_config_paths();
    for (par_from, par_to) in par_map {
        extract(&par_from, &par_to, &mut config_paths);
    }
    if !config_paths.is_empty() {
        panic!(
            "Error: The examples corresponding to the following config files \
             were not encountered in the pre-processing step:\n{}This is most \
             likely because the line numbers of the config files are not in \
             sync with the line numbers of the corresponding code blocks in \
             the latest versions of the Rust books. Please update the line \
             numbers of the config files and rerun the program.",
            paths_to_string(config_paths)
        );
    }
}

/// Returns a set of paths to the config files for examples in the Rust books.
fn get_config_paths() -> HashSet<PathBuf> {
    let config_dir: PathBuf = ["src", "tools", "dashboard", "configs"].iter().collect();
    let mut config_paths = HashSet::new();
    for entry in WalkDir::new(config_dir) {
        let entry = entry.unwrap().into_path();
        if entry.is_file() {
            config_paths.insert(entry);
        }
    }
    config_paths
}

/// Pretty prints the `paths` set.
fn paths_to_string(paths: HashSet<PathBuf>) -> String {
    let mut f = String::new();
    for path in paths {
        f.write_fmt(format_args!("    {:?}\n", path.to_str().unwrap())).unwrap();
    }
    f
}

/// Creates a new [`Tree`] from `path`, and a test `result`.
fn tree_from_path(mut path: Vec<String>, result: bool) -> dashboard::Tree {
    assert!(path.len() > 0, "Error: `path` must contain at least 1 element.");
    let mut tree = dashboard::Tree::new(
        dashboard::Node::new(
            path.pop().unwrap(),
            if result { 1 } else { 0 },
            if result { 0 } else { 1 },
        ),
        vec![],
    );
    for _ in 0..path.len() {
        tree = dashboard::Tree::new(
            dashboard::Node::new(path.pop().unwrap(), tree.data.num_pass, tree.data.num_fail),
            vec![tree],
        );
    }
    tree
}


/// Parses a `litani` run and generates a dashboard tree from it
fn parse_litani_output(path: &Path) -> dashboard::Tree {
    let file = fs::File::open(path).unwrap();
    let reader = BufReader::new(file);
    let run: LitaniRun = serde_json::from_reader(reader).unwrap();
    let mut tests =
        dashboard::Tree::new(dashboard::Node::new(String::from("dashboard"), 0, 0), vec![]);
    let pipelines = run.get_pipelines();
    for pipeline in pipelines {
        let (ns, l) = parse_log_line(&pipeline);
        tests = dashboard::Tree::merge(tests, tree_from_path(ns, l)).unwrap();
    }
    tests
}

/// Parses a `litani` pipeline and returns a pair containing
/// the path to a test and its result.
fn parse_log_line(pipeline: &LitaniPipeline) -> (Vec<String>, bool) {
    let l = pipeline.get_status();
    let name = pipeline.get_name();
    let mut ns: Vec<String> = name.split(&['/', '.'][..]).map(String::from).collect();
    // Remove unnecessary items from the path until "dashboard"
    let dash_index = ns.iter().position(|item| item == "dashboard").unwrap();
    ns.drain(..dash_index);
    // Remove unnecessary "rs" suffix.
    ns.pop();
    (ns, l)
}

/// Format and write a text version of the dashboard
fn generate_text_dashboard(dashboard: dashboard::Tree, path: &Path) {
    let dashboard_str =
        format!("# of tests: {}\t✔️ {}\t❌ {}\n{}",
                dashboard.data.num_pass + dashboard.data.num_fail,
                dashboard.data.num_pass,
                dashboard.data.num_fail,
                dashboard
    );
    fs::write(&path, dashboard_str).expect("Error: Unable to write dashboard results");
}

/// Runs examples using Litani build.
fn litani_run_tests() {
    let output_prefix: PathBuf = ["build", "output"].iter().collect();
    let output_symlink: PathBuf = output_prefix.join("latest");
    let dashboard_dir: PathBuf = ["src", "test", "dashboard"].iter().collect();
    util::add_rmc_and_litani_to_path();
    let mut litani = Litani::init("RMC", &output_prefix, &output_symlink);
    // Run all tests under the `src/test/dashboard` directory.
    for entry in WalkDir::new(dashboard_dir) {
        let entry = entry.unwrap().into_path();
        if entry.is_file() {
            // Ensure that we parse only Rust files by checking their extension
            let entry_ext = &entry.extension().and_then(OsStr::to_str);
            if let Some("rs") = entry_ext {
                let test_props = util::parse_test_header(&entry);
                util::add_test_pipeline(&mut litani, &test_props);
            }
        }
    }
    litani.run_build();
}

/// Extracts examples from the Rust books, run them through RMC, and displays
/// their results in a terminal dashboard.
pub fn generate_dashboard() {
    let litani_log: PathBuf = ["build", "output", "latest", "run.json"].iter().collect();
    let text_dash: PathBuf = ["build", "output", "latest", "html", "dashboard.txt"].iter().collect();
    // Parse the chapter/section hierarchy for the books.
    let mut map = HashMap::new();
    map.extend(parse_reference_hierarchy());
    map.extend(parse_nomicon_hierarchy());
    map.extend(parse_unstable_book_hierarchy());
    map.extend(parse_rust_by_example_hierarchy());
    // Extract examples from the books, pre-process them, and save them
    // following the partial hierarchy in map.
    extract_examples(map);
    // Generate Litani's HTML dashboard
    litani_run_tests();
    // Parse Litani's output
    let dashboard = parse_litani_output(&litani_log);
    // Generate text dashboard
    generate_text_dashboard(dashboard, &text_dash);
}
