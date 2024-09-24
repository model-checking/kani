// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::Path;

use tracing::*;

use crate::common::{Config, KaniFailStep, Mode};

#[derive(Clone, Debug)]
pub struct TestProps {
    // Extra flags to pass to the compiler
    pub compile_flags: Vec<String>,
    // Extra flags to pass to Kani
    pub kani_flags: Vec<String>,
    // Extra flags to pass to CBMC
    pub cbmc_flags: Vec<String>,
    // The step where Kani is expected to fail
    pub kani_panic_step: Option<KaniFailStep>,
}

impl TestProps {
    pub fn new() -> Self {
        TestProps {
            compile_flags: vec![],
            kani_flags: vec![],
            cbmc_flags: vec![],
            kani_panic_step: None,
        }
    }

    pub fn from_file(testfile: &Path, config: &Config) -> Self {
        let mut props = TestProps::new();
        props.load_from(testfile, config);
        props.kani_flags.extend(config.extra_args.iter().cloned());
        props
    }

    /// Loads properties from `testfile` into `props`.
    fn load_from(&mut self, testfile: &Path, config: &Config) {
        let mut has_edition = false;
        if !testfile.is_dir() {
            let file = File::open(testfile).unwrap();

            iter_header(testfile, file, &mut |ln| {
                if let Some(flags) = config.parse_compile_flags(ln) {
                    self.compile_flags.extend(flags.split_whitespace().map(|s| s.to_owned()));
                }

                if let Some(flags) = config.parse_kani_flags(ln) {
                    self.kani_flags.extend(flags.split_whitespace().map(|s| s.to_owned()));
                }

                if let Some(flags) = config.parse_cbmc_flags(ln) {
                    self.cbmc_flags.extend(flags.split_whitespace().map(|s| s.to_owned()));
                }

                if let Some(edition) = config.parse_edition(ln) {
                    self.compile_flags.push(format!("--edition={edition}"));
                    has_edition = true;
                }

                self.update_kani_fail_mode(ln, config);
            });
        }

        if let (Some(edition), false) = (&config.edition, has_edition) {
            self.compile_flags.push(format!("--edition={edition}"));
        }
    }

    /// Checks if `ln` specifies which stage the test should fail on and updates
    /// Kani fail mode accordingly.
    fn update_kani_fail_mode(&mut self, ln: &str, config: &Config) {
        let kani_fail_step = config.parse_kani_step_fail_directive(ln);
        match (self.kani_panic_step, kani_fail_step) {
            (None, Some(_)) => self.kani_panic_step = kani_fail_step,
            (Some(_), Some(_)) => panic!("multiple `kani-*-fail` headers in a single test"),
            (_, None) => {}
        }
    }
}

fn iter_header<R: Read>(testfile: &Path, rdr: R, it: &mut dyn FnMut(&str)) {
    if testfile.is_dir() {
        return;
    }

    let comment = if testfile.extension().map(|e| e == "rs") == Some(true) { "//" } else { "#" };

    let mut rdr = BufReader::new(rdr);
    let mut ln = String::new();

    loop {
        ln.clear();
        if rdr.read_line(&mut ln).unwrap() == 0 {
            break;
        }

        // Assume that any directives will be found before the first
        // module or function. This doesn't seem to be an optimization
        // with a warm page cache. Maybe with a cold one.
        let ln = ln.trim();
        if ln.starts_with("fn") || ln.starts_with("mod") {
            return;
        } else if let Some(rest) = ln.strip_prefix(comment) {
            it(rest.trim_start());
        }
    }
}

impl Config {
    fn parse_compile_flags(&self, line: &str) -> Option<String> {
        self.parse_name_value_directive(line, "compile-flags")
    }

    /// Parses strings of the form `kani-*-fail` and returns the step at which
    /// Kani is expected to panic.
    fn parse_kani_step_fail_directive(&self, line: &str) -> Option<KaniFailStep> {
        let check_kani = |mode: &str| {
            if self.mode != Mode::Kani {
                panic!("`kani-{mode}-fail` header is only supported in Kani tests");
            }
        };
        if self.parse_name_directive(line, "kani-check-fail") {
            check_kani("check");
            Some(KaniFailStep::Check)
        } else if self.parse_name_directive(line, "kani-codegen-fail") {
            check_kani("codegen");
            Some(KaniFailStep::Codegen)
        } else if self.parse_name_directive(line, "kani-verify-fail") {
            check_kani("verify");
            Some(KaniFailStep::Verify)
        } else {
            None
        }
    }

    /// Parses strings of the form `// kani-flags: ...` and returns the options listed after `kani-flags:`
    fn parse_kani_flags(&self, line: &str) -> Option<String> {
        self.parse_name_value_directive(line, "kani-flags")
    }

    /// Parses strings of the form `// cbmc-flags: ...` and returns the options listed after `cbmc-flags:`
    fn parse_cbmc_flags(&self, line: &str) -> Option<String> {
        self.parse_name_value_directive(line, "cbmc-flags")
    }

    fn parse_name_directive(&self, line: &str, directive: &str) -> bool {
        // Ensure the directive is a whole word. Do not match "ignore-x86" when
        // the line says "ignore-x86_64".
        line.starts_with(directive)
            && matches!(line.as_bytes().get(directive.len()), None | Some(&b' ') | Some(&b':'))
    }

    pub fn parse_name_value_directive(&self, line: &str, directive: &str) -> Option<String> {
        let colon = directive.len();
        if line.starts_with(directive) && line.as_bytes().get(colon) == Some(&b':') {
            let value = line[(colon + 1)..].to_owned();
            debug!("{}: {}", directive, value);
            Some(value)
        } else {
            None
        }
    }

    fn parse_edition(&self, line: &str) -> Option<String> {
        self.parse_name_value_directive(line, "edition")
    }
}

pub fn make_test_description<R: Read>(
    config: &Config,
    name: test::TestName,
    path: &Path,
    src: R,
) -> test::TestDesc {
    let mut should_fail = false;

    // If the path to the test contains "fixme" or "ignore", skip it.
    let file_path = path.to_str().unwrap();
    let (mut ignore, mut ignore_message) = if file_path.contains("fixme") {
        (true, Some("fixme test"))
    } else if file_path.contains("ignore") {
        (true, Some("ignore test"))
    } else {
        (false, None)
    };

    // The `KaniFixme` mode runs tests that are ignored in the `kani` suite
    if config.mode == Mode::KaniFixme {
        let file_path = path.to_str().unwrap();

        // `file_path` is going to be `tests/kani-fixme/...` so we
        // need to extract the base name if we want to ignore it
        let test_name: Vec<&str> = file_path.rsplit('/').collect();
        let base_name = test_name[0];

        // If the base name does NOT contain "fixme" or "ignore", we skip it.
        // All "fixme" tests are expected to fail
        (ignore, ignore_message) = if base_name.contains("fixme") {
            (false, None)
        } else if base_name.contains("ignore") {
            (true, Some("ignore test"))
        } else {
            (true, Some("regular test"))
        };
        should_fail = true;
    }

    iter_header(path, src, &mut |ln| {
        should_fail |= config.parse_name_directive(ln, "should-fail");
    });

    // The `should-fail` annotation doesn't apply to pretty tests,
    // since we run the pretty printer across all tests by default.
    // If desired, we could add a `should-fail-pretty` annotation.
    let should_panic = match config.mode {
        _ if should_fail => test::ShouldPanic::Yes,
        _ => test::ShouldPanic::No,
    };

    test::TestDesc {
        name,
        ignore,
        ignore_message,
        should_panic,
        compile_fail: false,
        no_run: false,
        test_type: test::TestType::Unknown,
        // Enter dummy values since the test doesn't have a line per-se.
        source_file: "unknown_file",
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
    }
}
