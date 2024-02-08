// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Utilities to extract examples from Rust books, run them through Kani, and
//! display their results.

extern crate rustc_span;

use crate::{
    bookrunner,
    litani::{Litani, LitaniPipeline, LitaniRun},
    util::{self, FailStep, TestProps},
};
use inflector::cases::{snakecase::to_snake_case, titlecase::to_title_case};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use rustc_span::edition::Edition;
use rustdoc::{
    doctest::{make_test, Tester},
    html::markdown::{find_testable_code, ErrorCodes, Ignore, LangString},
};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt::Write,
    fs,
    io::BufReader,
    iter::FromIterator,
    path::{Path, PathBuf},
    str::FromStr,
};
use walkdir::WalkDir;

// Books may include a `SUMMARY.md` file or not. If they do, the info in
// `SummaryData` is helpful to parse the hierarchy, otherwise we use a
// `DirectoryData` structure

// Data needed for parsing a book with a summary file
struct SummaryData {
    // Path to the summary file
    summary_path: PathBuf,
    // Line that indicates the start of the summary section
    summary_start: String,
}

// Data needed for parsing book without a summary file
struct DirectoryData {
    // Directory to be processed, starting from root of the book
    src: PathBuf,
    // Directory where the examples extracted from the book should reside
    dest: PathBuf,
}

// Data structure representing a Rust book
struct Book {
    // Name of the book
    name: String,
    // Default Rust edition
    default_edition: Edition,
    // Data about the summary file
    summary_data: Option<SummaryData>,
    // Data about the source/destination directories
    directory_data: Option<DirectoryData>,
    // Path to the `book.toml` file
    toml_path: PathBuf,
    // The hierarchy map used for example extraction
    hierarchy: HashMap<PathBuf, PathBuf>,
}

impl Book {
    /// Parse the chapter/section hierarchy and set the default edition
    fn parse_hierarchy(&mut self) {
        if self.summary_data.is_some() {
            assert!(self.directory_data.is_none());
            self.parse_hierarchy_with_summary();
        } else {
            assert!(self.directory_data.is_some());
            self.parse_hierarchy_without_summary();
        }
        self.default_edition = self.get_rust_edition().unwrap_or(Edition::Edition2015);
    }

    /// Parses the chapter/section hierarchy in the markdown file specified by
    /// `summary_path` and returns a mapping from markdown files containing rust
    /// code to corresponding directories where the extracted rust code should
    /// reside.
    fn parse_hierarchy_with_summary(&mut self) {
        let summary_path = &self.summary_data.as_ref().unwrap().summary_path;
        let summary_start = &self.summary_data.as_ref().unwrap().summary_start;
        let summary_dir = summary_path.parent().unwrap().to_path_buf();
        let summary = fs::read_to_string(summary_path.clone()).unwrap();
        assert!(
            summary.starts_with(summary_start.as_str()),
            "Error: The start of {} summary file changed.",
            self.name
        );
        // Skip the `start` of the summary.
        let n = Parser::new(summary_start.as_str()).count();
        let parser = Parser::new(&summary).skip(n);
        // Set `self.name` as the root of the hierarchical path.
        let mut hierarchy_path: PathBuf =
            ["tests", "bookrunner", "books", self.name.as_str()].iter().collect();
        let mut prev_event_is_text_or_code = false;
        let mut current_link_url = String::from("");
        for event in parser {
            match event {
                Event::End(TagEnd::Item) => {
                    // Pop the current chapter/section from the hierarchy once
                    // we are done processing it and its subsections.
                    hierarchy_path.pop();
                    prev_event_is_text_or_code = false;
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    current_link_url = dest_url.into_string();
                }
                Event::End(TagEnd::Link) => {
                    // At the start of the link tag, the hierarchy does not yet
                    // contain the title of the current chapter/section. So, we wait
                    // for the end of the link tag before adding the path and
                    // hierarchy of the current chapter/section to the map.
                    let mut full_path = summary_dir.clone();
                    full_path.extend(current_link_url.split('/'));
                    self.hierarchy.insert(full_path, hierarchy_path.clone());
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
                        let prev_text =
                            hierarchy_path.file_name().unwrap().to_str().unwrap().to_string();
                        hierarchy_path.pop();
                        hierarchy_path.push(format!("{prev_text}{text}"));
                    } else {
                        // If not, add the current title to the hierarchy.
                        hierarchy_path.push(&text);
                    }
                    prev_event_is_text_or_code = true;
                }
                _ => (),
            }
        }
    }

    /// Parses books that do not have a `SUMMARY.md` file (i.e., a table of
    /// contents). We parse them manually and make a "best effort" to make it
    /// look like the online version.
    fn parse_hierarchy_without_summary(&mut self) {
        let directory_data = self.directory_data.as_ref().unwrap();
        let src = &directory_data.src;
        let dest = &directory_data.dest;
        let mut src_prefix: PathBuf = src.clone();
        let mut dest_prefix: PathBuf = dest.clone();
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
                        self.hierarchy
                            .insert(entry.clone(), dest_prefix.join(to_title_case(entry_stem)));
                    } else {
                        // Otherwise, follow the book's snake case format for files.
                        self.hierarchy
                            .insert(entry.clone(), dest_prefix.join(to_snake_case(entry_stem)));
                    }
                }
            }
        }
    }

    // Get the Rust edition from the `book.toml` file
    fn get_rust_edition(&self) -> Option<Edition> {
        let file = fs::read_to_string(&self.toml_path).unwrap();
        let toml_data: toml::Value = toml::from_str(&file).unwrap();
        // The Rust edition is specified in the `rust.edition` attribute
        let rust_block = toml_data.get("rust")?;
        let edition_attr = rust_block.get("edition")?;
        let edition_str = edition_attr.as_str()?;
        Some(Edition::from_str(edition_str).unwrap())
    }

    /// Extracts examples from the markdown files specified by each key in the given
    /// `map`, pre-processes those examples, and saves them in the directory
    /// specified by the corresponding value.
    fn extract_examples(&self) {
        let mut config_paths = get_config_paths(self.name.as_str());
        for (par_from, par_to) in &self.hierarchy {
            extract(par_from, par_to, &mut config_paths, self.default_edition);
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
}
/// Set up [The Rust Reference](https://doc.rust-lang.org/nightly/reference)
/// book.
fn setup_reference_book() -> Book {
    let summary_data = SummaryData {
        summary_start: "# The Rust Reference\n\n[Introduction](introduction.md)".to_string(),
        summary_path: ["tools", "bookrunner", "rust-doc", "reference", "src", "SUMMARY.md"]
            .iter()
            .collect(),
    };
    Book {
        name: "The Rust Reference".to_string(),
        summary_data: Some(summary_data),
        directory_data: None,
        toml_path: ["tools", "bookrunner", "rust-doc", "reference", "book.toml"].iter().collect(),
        hierarchy: HashMap::from_iter([(
            ["tools", "bookrunner", "rust-doc", "reference", "src", "introduction.md"]
                .iter()
                .collect(),
            ["tests", "bookrunner", "books", "The Rust Reference", "Introduction"].iter().collect(),
        )]),
        default_edition: Edition::Edition2015,
    }
}

/// Set up [The Rustonomicon](https://doc.rust-lang.org/nightly/nomicon) book.
fn setup_nomicon_book() -> Book {
    let summary_data = SummaryData {
        summary_path: ["tools", "bookrunner", "rust-doc", "nomicon", "src", "SUMMARY.md"]
            .iter()
            .collect(),
        summary_start: "# Summary\n\n[Introduction](intro.md)".to_string(),
    };
    Book {
        name: "The Rustonomicon".to_string(),
        summary_data: Some(summary_data),
        directory_data: None,
        toml_path: ["tools", "bookrunner", "rust-doc", "nomicon", "book.toml"].iter().collect(),
        hierarchy: HashMap::from_iter([(
            ["tools", "bookrunner", "rust-doc", "nomicon", "src", "intro.md"].iter().collect(),
            ["tests", "bookrunner", "books", "The Rustonomicon", "Introduction"].iter().collect(),
        )]),
        default_edition: Edition::Edition2015,
    }
}

/// Set up the
/// [Rust Unstable Book](https://doc.rust-lang.org/beta/unstable-book/).
fn setup_unstable_book() -> Book {
    let directory_data = DirectoryData {
        src: ["tools", "bookrunner", "rust-doc", "unstable-book", "src"].iter().collect(),
        dest: ["tests", "bookrunner", "books", "The Unstable Book"].iter().collect(),
    };
    Book {
        name: "The Rust Unstable Book".to_string(),
        summary_data: None,
        directory_data: Some(directory_data),
        toml_path: ["tools", "bookrunner", "rust-doc", "unstable-book", "book.toml"]
            .iter()
            .collect(),
        hierarchy: HashMap::new(),
        default_edition: Edition::Edition2015,
    }
}

/// Set up the
/// [Rust by Example](https://doc.rust-lang.org/nightly/rust-by-example) book.
fn setup_rust_by_example_book() -> Book {
    let summary_data = SummaryData {
        summary_path: ["tools", "bookrunner", "rust-doc", "rust-by-example", "src", "SUMMARY.md"]
            .iter()
            .collect(),
        summary_start: "# Summary\n\n[Introduction](index.md)".to_string(),
    };
    Book {
        name: "Rust by Example".to_string(),
        summary_data: Some(summary_data),
        directory_data: None,
        toml_path: ["tools", "bookrunner", "rust-doc", "rust-by-example", "book.toml"]
            .iter()
            .collect(),
        hierarchy: HashMap::from_iter([(
            ["tools", "bookrunner", "rust-doc", "rust-by-example", "src", "index.md"]
                .iter()
                .collect(),
            ["tests", "bookrunner", "books", "Rust by Example", "Introduction"].iter().collect(),
        )]),
        default_edition: Edition::Edition2015,
    }
}

/// This data structure contains the code and configs of an example in the Rust books.
struct Example {
    /// The example code extracted from a codeblock.
    code: String,
    // Line number of the code block.
    line: usize,
    // Configurations in the header of the codeblock.
    config: LangString,
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
    let config_dir: PathBuf = ["tools", "bookrunner", "configs"].iter().collect();
    let test_dir: PathBuf = ["tests", "bookrunner"].iter().collect();
    // `path` has the following form:
    // `tests/bookrunner/books/<hierarchy>
    // If `example` has a custom diff file, the path to the diff file will have
    // the following form:
    // `tools/bookrunner/configs/books/<hierarchy>/<example.line>.diff`
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
    let config_dir: PathBuf = ["tools", "bookrunner", "configs"].iter().collect();
    let test_dir: PathBuf = ["tests", "bookrunner"].iter().collect();
    // `path` has the following form:
    // `tests/bookrunner/books/<hierarchy>
    // If `example` has a custom props file, the path to the props file will
    // have the following form:
    // `tools/bookrunner/configs/books/<hierarchy>/<example.line>.props`
    // where <hierarchy> is the same for both paths.
    let mut props_path = config_dir.join(path.strip_prefix(&test_dir).unwrap());
    props_path.extend_one(format!("{}.props", example.line));
    let mut props = if props_path.exists() {
        config_paths.remove(&props_path);
        util::parse_test_header(&props_path)
    } else {
        TestProps::new(path.to_path_buf(), None, Vec::new(), Vec::new())
    };
    // Add edition flag to the example
    let edition_year = format!("{}", example.config.edition.unwrap());
    props.rustc_args.push(String::from("--edition"));
    props.rustc_args.push(edition_year);

    if props.fail_step.is_none() {
        if example.config.compile_fail {
            // Most examples with `compile_fail` annotation fail because of
            // check errors. This heuristic can be overridden by manually
            //specifying the fail step in the corresponding config file.
            props.fail_step = Some(FailStep::Check);
        } else if example.config.should_panic {
            // Kani should catch run-time errors.
            props.fail_step = Some(FailStep::Verification);
        }
    }
    example.code = format!("{props}{}", example.code);
}

/// Extracts examples from the markdown file specified by `par_from`,
/// pre-processes those examples, and saves them in the directory specified by
/// `par_to`.
fn extract(
    par_from: &Path,
    par_to: &Path,
    config_paths: &mut HashSet<PathBuf>,
    default_edition: Edition,
) {
    let code = fs::read_to_string(par_from).unwrap();
    let mut examples = Examples(Vec::new());
    find_testable_code(&code, &mut examples, ErrorCodes::No, false, None);
    for mut example in examples.0 {
        apply_diff(par_to, &mut example, config_paths);
        example.config.edition = Some(example.config.edition.unwrap_or(default_edition));
        example.code = make_test(
            &example.code,
            None,
            false,
            &Default::default(),
            example.config.edition.unwrap(),
            None,
        )
        .0;
        prepend_props(par_to, &mut example, config_paths);
        let rs_path = par_to.join(format!("{}.rs", example.line));
        fs::create_dir_all(rs_path.parent().unwrap()).unwrap();
        fs::write(rs_path, example.code).unwrap();
    }
}

/// Returns a set of paths to the config files for examples in the Rust books.
fn get_config_paths(book_name: &str) -> HashSet<PathBuf> {
    let config_dir: PathBuf =
        ["tools", "bookrunner", "configs", "books", book_name].iter().collect();
    let mut config_paths = HashSet::new();
    if config_dir.exists() {
        for entry in WalkDir::new(config_dir) {
            let entry = entry.unwrap().into_path();
            if entry.is_file() {
                config_paths.insert(entry);
            }
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

/// Creates a new [`bookrunner::Tree`] from `path`, and a test `result`.
fn tree_from_path(mut path: Vec<String>, result: bool) -> bookrunner::Tree {
    assert!(!path.is_empty(), "Error: `path` must contain at least 1 element.");
    let mut tree = bookrunner::Tree::new(
        bookrunner::Node::new(
            path.pop().unwrap(),
            if result { 1 } else { 0 },
            if result { 0 } else { 1 },
        ),
        vec![],
    );
    for _ in 0..path.len() {
        tree = bookrunner::Tree::new(
            bookrunner::Node::new(path.pop().unwrap(), tree.data.num_pass, tree.data.num_fail),
            vec![tree],
        );
    }
    tree
}

/// Parses a `litani` run and generates a bookrunner tree from it
fn parse_litani_output(path: &Path) -> bookrunner::Tree {
    let file = fs::File::open(path).unwrap();
    let reader = BufReader::new(file);
    let run: LitaniRun = serde_json::from_reader(reader).unwrap();
    let mut tests =
        bookrunner::Tree::new(bookrunner::Node::new(String::from("bookrunner"), 0, 0), vec![]);
    let pipelines = run.get_pipelines();
    for pipeline in pipelines {
        let (ns, l) = parse_log_line(&pipeline);
        tests = bookrunner::Tree::merge(tests, tree_from_path(ns, l)).unwrap();
    }
    tests
}

/// Parses a `litani` pipeline and returns a pair containing
/// the path to a test and its result.
fn parse_log_line(pipeline: &LitaniPipeline) -> (Vec<String>, bool) {
    let l = pipeline.get_status();
    let name = pipeline.get_name();
    let mut ns: Vec<String> = name.split(&['/', '.'][..]).map(String::from).collect();
    // Remove unnecessary items from the path until "bookrunner"
    let dash_index = ns.iter().position(|item| item == "bookrunner").unwrap();
    ns.drain(..dash_index);
    // Remove unnecessary "rs" suffix.
    ns.pop();
    (ns, l)
}

/// Format and write a text version of the bookrunner report
fn generate_text_bookrunner(bookrunner: bookrunner::Tree, path: &Path) {
    let bookrunner_str = format!(
        "# of tests: {}\t✔️ {}\t❌ {}\n{}",
        bookrunner.data.num_pass + bookrunner.data.num_fail,
        bookrunner.data.num_pass,
        bookrunner.data.num_fail,
        bookrunner
    );
    fs::write(path, bookrunner_str).expect("Error: Unable to write bookrunner results");
}

/// Runs examples using Litani build.
fn litani_run_tests() {
    let output_prefix: PathBuf = ["build", "output"].iter().collect();
    let output_symlink: PathBuf = output_prefix.join("latest");
    let bookrunner_dir: PathBuf = ["tests", "bookrunner"].iter().collect();
    let stage_names = ["check", "codegen", "verification"];

    util::add_kani_to_path();
    let mut litani = Litani::init("Book Runner", &stage_names, &output_prefix, &output_symlink);

    // Run all tests under the `tests/bookrunner` directory.
    for entry in WalkDir::new(bookrunner_dir) {
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

/// Extracts examples from the Rust books, run them through Kani, and displays
/// their results in a HTML webpage.
pub fn generate_run() {
    let litani_log: PathBuf = ["build", "output", "latest", "run.json"].iter().collect();
    let text_dash: PathBuf =
        ["build", "output", "latest", "html", "bookrunner.txt"].iter().collect();
    // Set up books
    let books: Vec<Book> = vec![
        setup_reference_book(),
        setup_nomicon_book(),
        setup_unstable_book(),
        setup_rust_by_example_book(),
    ];
    for mut book in books {
        // Parse the chapter/section hierarchy
        book.parse_hierarchy();
        // Extract examples, pre-process them, and save them according to the
        // parsed hierarchy
        book.extract_examples();
    }
    // Generate Litani's HTML bookrunner
    litani_run_tests();
    // Parse Litani's output
    let bookrunner = parse_litani_output(&litani_log);
    // Generate text version
    generate_text_bookrunner(bookrunner, &text_dash);
}
