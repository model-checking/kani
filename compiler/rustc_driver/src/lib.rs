//! The Rust compiler.
//!
//! # Note
//!
//! This API is completely unstable and subject to change.

#![doc(html_root_url = "https://doc.rust-lang.org/nightly/nightly-rustc/")]
#![feature(nll)]
#![feature(once_cell)]
#![recursion_limit = "256"]

#[macro_use]
extern crate tracing;

pub extern crate rustc_plugin_impl as plugin;

use rustc_ast as ast;
use rustc_codegen_ssa::{traits::CodegenBackend, CodegenResults};
use rustc_data_structures::profiling::{get_resident_set_size, print_time_passes_entry};
use rustc_data_structures::sync::SeqCst;
use rustc_errors::registry::{InvalidErrorCode, Registry};
use rustc_errors::{ErrorReported, PResult};
use rustc_feature::find_gated_cfg;
use rustc_interface::util::{self, collect_crate_types, get_codegen_backend};
use rustc_interface::{interface, Queries};
use rustc_lint::LintStore;
use rustc_log::stdout_isatty;
use rustc_metadata::locator;
use rustc_save_analysis as save;
use rustc_save_analysis::DumpHandler;
use rustc_serialize::json::{self, ToJson};
use rustc_session::config::{nightly_options, CG_OPTIONS, DB_OPTIONS};
use rustc_session::config::{ErrorOutputType, Input, OutputType, PrintRequest, TrimmedDefPaths};
use rustc_session::cstore::MetadataLoader;
use rustc_session::getopts;
use rustc_session::lint::{Lint, LintId};
use rustc_session::{config, DiagnosticOutput, Session};
use rustc_session::{early_error, early_error_no_abort, early_warn};
use rustc_span::source_map::{FileLoader, FileName};
use rustc_span::symbol::sym;

use std::borrow::Cow;
use std::cmp::max;
use std::default::Default;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::lazy::SyncLazy;
use std::panic::{self, catch_unwind};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::str;
use std::time::Instant;

pub mod args;
pub mod pretty;

/// Exit status code used for successful compilation and help output.
pub const EXIT_SUCCESS: i32 = 0;

/// Exit status code used for compilation failures and invalid flags.
pub const EXIT_FAILURE: i32 = 1;

const BUG_REPORT_URL: &str = "https://github.com/rust-lang/rust/issues/new\
    ?labels=C-bug%2C+I-ICE%2C+T-compiler&template=ice.md";

const ICE_REPORT_COMPILER_FLAGS: &[&str] = &["Z", "C", "crate-type"];

const ICE_REPORT_COMPILER_FLAGS_EXCLUDE: &[&str] = &["metadata", "extra-filename"];

const ICE_REPORT_COMPILER_FLAGS_STRIP_VALUE: &[&str] = &["incremental"];

pub fn abort_on_err<T>(result: Result<T, ErrorReported>, sess: &Session) -> T {
    match result {
        Err(..) => {
            sess.abort_if_errors();
            panic!("error reported but abort_if_errors didn't abort???");
        }
        Ok(x) => x,
    }
}

pub trait Callbacks {
    /// Called before creating the compiler instance
    fn config(&mut self, _config: &mut interface::Config) {}
    /// Called after parsing. Return value instructs the compiler whether to
    /// continue the compilation afterwards (defaults to `Compilation::Continue`)
    fn after_parsing<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        _queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        Compilation::Continue
    }
    /// Called after expansion. Return value instructs the compiler whether to
    /// continue the compilation afterwards (defaults to `Compilation::Continue`)
    fn after_expansion<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        _queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        Compilation::Continue
    }
    /// Called after analysis. Return value instructs the compiler whether to
    /// continue the compilation afterwards (defaults to `Compilation::Continue`)
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        _queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        Compilation::Continue
    }
}

#[derive(Default)]
pub struct TimePassesCallbacks {
    time_passes: bool,
}

impl Callbacks for TimePassesCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        // If a --prints=... option has been given, we don't print the "total"
        // time because it will mess up the --prints output. See #64339.
        self.time_passes = config.opts.prints.is_empty()
            && (config.opts.debugging_opts.time_passes || config.opts.debugging_opts.time);
        config.opts.trimmed_def_paths = TrimmedDefPaths::GoodPath;
    }
}

pub fn diagnostics_registry() -> Registry {
    Registry::new(rustc_error_codes::DIAGNOSTICS)
}

/// This is the primary entry point for rustc.
pub struct RunCompiler<'a, 'b> {
    at_args: &'a [String],
    callbacks: &'b mut (dyn Callbacks + Send),
    file_loader: Option<Box<dyn FileLoader + Send + Sync>>,
    emitter: Option<Box<dyn Write + Send>>,
    make_codegen_backend:
        Option<Box<dyn FnOnce(&config::Options) -> Box<dyn CodegenBackend> + Send>>,
}

impl<'a, 'b> RunCompiler<'a, 'b> {
    pub fn new(at_args: &'a [String], callbacks: &'b mut (dyn Callbacks + Send)) -> Self {
        Self { at_args, callbacks, file_loader: None, emitter: None, make_codegen_backend: None }
    }

    /// Set a custom codegen backend.
    ///
    /// Used by cg_clif.
    pub fn set_make_codegen_backend(
        &mut self,
        make_codegen_backend: Option<
            Box<dyn FnOnce(&config::Options) -> Box<dyn CodegenBackend> + Send>,
        >,
    ) -> &mut Self {
        self.make_codegen_backend = make_codegen_backend;
        self
    }

    /// Emit diagnostics to the specified location.
    ///
    /// Used by RLS.
    pub fn set_emitter(&mut self, emitter: Option<Box<dyn Write + Send>>) -> &mut Self {
        self.emitter = emitter;
        self
    }

    /// Load files from sources other than the file system.
    ///
    /// Used by RLS.
    pub fn set_file_loader(
        &mut self,
        file_loader: Option<Box<dyn FileLoader + Send + Sync>>,
    ) -> &mut Self {
        self.file_loader = file_loader;
        self
    }

    /// Parse args and run the compiler.
    pub fn run(self) -> interface::Result<()> {
        run_compiler(
            self.at_args,
            self.callbacks,
            self.file_loader,
            self.emitter,
            self.make_codegen_backend,
        )
    }
}
fn run_compiler(
    at_args: &[String],
    callbacks: &mut (dyn Callbacks + Send),
    file_loader: Option<Box<dyn FileLoader + Send + Sync>>,
    emitter: Option<Box<dyn Write + Send>>,
    make_codegen_backend: Option<
        Box<dyn FnOnce(&config::Options) -> Box<dyn CodegenBackend> + Send>,
    >,
) -> interface::Result<()> {
    let args = args::arg_expand_all(at_args);

    let diagnostic_output = emitter.map_or(DiagnosticOutput::Default, DiagnosticOutput::Raw);
    let matches = match handle_options(&args) {
        Some(matches) => matches,
        None => return Ok(()),
    };

    let sopts = config::build_session_options(&matches);

    if let Some(ref code) = matches.opt_str("explain") {
        handle_explain(diagnostics_registry(), code, sopts.error_format);
        return Ok(());
    }

    let cfg = interface::parse_cfgspecs(matches.opt_strs("cfg"));
    let (odir, ofile) = make_output(&matches);
    let mut config = interface::Config {
        opts: sopts,
        crate_cfg: cfg,
        input: Input::File(PathBuf::new()),
        input_path: None,
        output_file: ofile,
        output_dir: odir,
        file_loader,
        diagnostic_output,
        stderr: None,
        lint_caps: Default::default(),
        parse_sess_created: None,
        register_lints: None,
        override_queries: None,
        make_codegen_backend,
        registry: diagnostics_registry(),
    };

    match make_input(config.opts.error_format, &matches.free) {
        Err(ErrorReported) => return Err(ErrorReported),
        Ok(Some((input, input_file_path))) => {
            config.input = input;
            config.input_path = input_file_path;

            callbacks.config(&mut config);
        }
        Ok(None) => match matches.free.len() {
            0 => {
                callbacks.config(&mut config);
                interface::run_compiler(config, |compiler| {
                    let sopts = &compiler.session().opts;
                    if sopts.describe_lints {
                        let mut lint_store = rustc_lint::new_lint_store(
                            sopts.debugging_opts.no_interleave_lints,
                            compiler.session().unstable_options(),
                        );
                        let registered_lints =
                            if let Some(register_lints) = compiler.register_lints() {
                                register_lints(compiler.session(), &mut lint_store);
                                true
                            } else {
                                false
                            };
                        describe_lints(compiler.session(), &lint_store, registered_lints);
                        return;
                    }
                    let should_stop = RustcDefaultCalls::print_crate_info(
                        &***compiler.codegen_backend(),
                        compiler.session(),
                        None,
                        compiler.output_dir(),
                        compiler.output_file(),
                        compiler.temps_dir(),
                    );

                    if should_stop == Compilation::Stop {
                        return;
                    }
                    early_error(sopts.error_format, "no input filename given")
                });
                return Ok(());
            }
            1 => panic!("make_input should have provided valid inputs"),
            _ => early_error(
                config.opts.error_format,
                &format!(
                    "multiple input filenames provided (first two filenames are `{}` and `{}`)",
                    matches.free[0], matches.free[1],
                ),
            ),
        },
    };

    interface::run_compiler(config, |compiler| {
        let sess = compiler.session();
        let should_stop = RustcDefaultCalls::print_crate_info(
            &***compiler.codegen_backend(),
            sess,
            Some(compiler.input()),
            compiler.output_dir(),
            compiler.output_file(),
            compiler.temps_dir(),
        )
        .and_then(|| {
            RustcDefaultCalls::list_metadata(
                sess,
                &*compiler.codegen_backend().metadata_loader(),
                compiler.input(),
            )
        })
        .and_then(|| RustcDefaultCalls::try_process_rlink(sess, compiler));

        if should_stop == Compilation::Stop {
            return sess.compile_status();
        }

        let linker = compiler.enter(|queries| {
            let early_exit = || sess.compile_status().map(|_| None);
            queries.parse()?;

            if let Some(ppm) = &sess.opts.pretty {
                if ppm.needs_ast_map() {
                    let expanded_crate = queries.expansion()?.peek().0.clone();
                    queries.global_ctxt()?.peek_mut().enter(|tcx| {
                        pretty::print_after_hir_lowering(
                            tcx,
                            compiler.input(),
                            &*expanded_crate,
                            *ppm,
                            compiler.output_file().as_ref().map(|p| &**p),
                        );
                        Ok(())
                    })?;
                } else {
                    let krate = queries.parse()?.take();
                    pretty::print_after_parsing(
                        sess,
                        compiler.input(),
                        &krate,
                        *ppm,
                        compiler.output_file().as_ref().map(|p| &**p),
                    );
                }
                trace!("finished pretty-printing");
                return early_exit();
            }

            if callbacks.after_parsing(compiler, queries) == Compilation::Stop {
                return early_exit();
            }

            if sess.opts.debugging_opts.parse_only
                || sess.opts.debugging_opts.show_span.is_some()
                || sess.opts.debugging_opts.ast_json_noexpand
            {
                return early_exit();
            }

            {
                let (_, lint_store) = &*queries.register_plugins()?.peek();

                // Lint plugins are registered; now we can process command line flags.
                if sess.opts.describe_lints {
                    describe_lints(sess, lint_store, true);
                    return early_exit();
                }
            }

            queries.expansion()?;
            if callbacks.after_expansion(compiler, queries) == Compilation::Stop {
                return early_exit();
            }

            queries.prepare_outputs()?;

            if sess.opts.output_types.contains_key(&OutputType::DepInfo)
                && sess.opts.output_types.len() == 1
            {
                return early_exit();
            }

            queries.global_ctxt()?;

            if sess.opts.debugging_opts.no_analysis || sess.opts.debugging_opts.ast_json {
                return early_exit();
            }

            queries.global_ctxt()?.peek_mut().enter(|tcx| {
                let result = tcx.analysis(());
                if sess.opts.debugging_opts.save_analysis {
                    let crate_name = queries.crate_name()?.peek().clone();
                    sess.time("save_analysis", || {
                        save::process_crate(
                            tcx,
                            &crate_name,
                            compiler.input(),
                            None,
                            DumpHandler::new(
                                compiler.output_dir().as_ref().map(|p| &**p),
                                &crate_name,
                            ),
                        )
                    });
                }
                result
            })?;

            if callbacks.after_analysis(compiler, queries) == Compilation::Stop {
                return early_exit();
            }

            queries.ongoing_codegen()?;

            if sess.opts.debugging_opts.print_type_sizes {
                sess.code_stats.print_type_sizes();
            }

            let linker = queries.linker()?;
            Ok(Some(linker))
        })?;

        if let Some(linker) = linker {
            let _timer = sess.timer("link");
            linker.link()?
        }

        if sess.opts.debugging_opts.perf_stats {
            sess.print_perf_stats();
        }

        if sess.opts.debugging_opts.print_fuel.is_some() {
            eprintln!(
                "Fuel used by {}: {}",
                sess.opts.debugging_opts.print_fuel.as_ref().unwrap(),
                sess.print_fuel.load(SeqCst)
            );
        }

        Ok(())
    })
}

#[cfg(unix)]
pub fn set_sigpipe_handler() {
    unsafe {
        // Set the SIGPIPE signal handler, so that an EPIPE
        // will cause rustc to terminate, as expected.
        assert_ne!(libc::signal(libc::SIGPIPE, libc::SIG_DFL), libc::SIG_ERR);
    }
}

#[cfg(windows)]
pub fn set_sigpipe_handler() {}

// Extract output directory and file from matches.
fn make_output(matches: &getopts::Matches) -> (Option<PathBuf>, Option<PathBuf>) {
    let odir = matches.opt_str("out-dir").map(|o| PathBuf::from(&o));
    let ofile = matches.opt_str("o").map(|o| PathBuf::from(&o));
    (odir, ofile)
}

// Extract input (string or file and optional path) from matches.
fn make_input(
    error_format: ErrorOutputType,
    free_matches: &[String],
) -> Result<Option<(Input, Option<PathBuf>)>, ErrorReported> {
    if free_matches.len() == 1 {
        let ifile = &free_matches[0];
        if ifile == "-" {
            let mut src = String::new();
            if io::stdin().read_to_string(&mut src).is_err() {
                // Immediately stop compilation if there was an issue reading
                // the input (for example if the input stream is not UTF-8).
                early_error_no_abort(
                    error_format,
                    "couldn't read from stdin, as it did not contain valid UTF-8",
                );
                return Err(ErrorReported);
            }
            if let Ok(path) = env::var("UNSTABLE_RUSTDOC_TEST_PATH") {
                let line = env::var("UNSTABLE_RUSTDOC_TEST_LINE").expect(
                    "when UNSTABLE_RUSTDOC_TEST_PATH is set \
                                    UNSTABLE_RUSTDOC_TEST_LINE also needs to be set",
                );
                let line = isize::from_str_radix(&line, 10)
                    .expect("UNSTABLE_RUSTDOC_TEST_LINE needs to be an number");
                let file_name = FileName::doc_test_source_code(PathBuf::from(path), line);
                Ok(Some((Input::Str { name: file_name, input: src }, None)))
            } else {
                Ok(Some((Input::Str { name: FileName::anon_source_code(&src), input: src }, None)))
            }
        } else {
            Ok(Some((Input::File(PathBuf::from(ifile)), Some(PathBuf::from(ifile)))))
        }
    } else {
        Ok(None)
    }
}

/// Whether to stop or continue compilation.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Compilation {
    Stop,
    Continue,
}

impl Compilation {
    pub fn and_then<F: FnOnce() -> Compilation>(self, next: F) -> Compilation {
        match self {
            Compilation::Stop => Compilation::Stop,
            Compilation::Continue => next(),
        }
    }
}

/// CompilerCalls instance for a regular rustc build.
#[derive(Copy, Clone)]
pub struct RustcDefaultCalls;

fn handle_explain(registry: Registry, code: &str, output: ErrorOutputType) {
    let upper_cased_code = code.to_ascii_uppercase();
    let normalised = if upper_cased_code.starts_with('E') {
        upper_cased_code
    } else {
        format!("E{0:0>4}", code)
    };
    match registry.try_find_description(&normalised) {
        Ok(Some(description)) => {
            let mut is_in_code_block = false;
            let mut text = String::new();
            // Slice off the leading newline and print.
            for line in description.lines() {
                let indent_level =
                    line.find(|c: char| !c.is_whitespace()).unwrap_or_else(|| line.len());
                let dedented_line = &line[indent_level..];
                if dedented_line.starts_with("```") {
                    is_in_code_block = !is_in_code_block;
                    text.push_str(&line[..(indent_level + 3)]);
                } else if is_in_code_block && dedented_line.starts_with("# ") {
                    continue;
                } else {
                    text.push_str(line);
                }
                text.push('\n');
            }
            if stdout_isatty() {
                show_content_with_pager(&text);
            } else {
                print!("{}", text);
            }
        }
        Ok(None) => {
            early_error(output, &format!("no extended information for {}", code));
        }
        Err(InvalidErrorCode) => {
            early_error(output, &format!("{} is not a valid error code", code));
        }
    }
}

fn show_content_with_pager(content: &str) {
    let pager_name = env::var_os("PAGER").unwrap_or_else(|| {
        if cfg!(windows) { OsString::from("more.com") } else { OsString::from("less") }
    });

    let mut fallback_to_println = false;

    match Command::new(pager_name).stdin(Stdio::piped()).spawn() {
        Ok(mut pager) => {
            if let Some(pipe) = pager.stdin.as_mut() {
                if pipe.write_all(content.as_bytes()).is_err() {
                    fallback_to_println = true;
                }
            }

            if pager.wait().is_err() {
                fallback_to_println = true;
            }
        }
        Err(_) => {
            fallback_to_println = true;
        }
    }

    // If pager fails for whatever reason, we should still print the content
    // to standard output
    if fallback_to_println {
        print!("{}", content);
    }
}

impl RustcDefaultCalls {
    pub fn try_process_rlink(sess: &Session, compiler: &interface::Compiler) -> Compilation {
        if sess.opts.debugging_opts.link_only {
            if let Input::File(file) = compiler.input() {
                // FIXME: #![crate_type] and #![crate_name] support not implemented yet
                sess.init_crate_types(collect_crate_types(sess, &[]));
                let outputs = compiler.build_output_filenames(sess, &[]);
                let rlink_data = fs::read_to_string(file).unwrap_or_else(|err| {
                    sess.fatal(&format!("failed to read rlink file: {}", err));
                });
                let codegen_results: CodegenResults =
                    json::decode(&rlink_data).unwrap_or_else(|err| {
                        sess.fatal(&format!("failed to decode rlink: {}", err));
                    });
                let result = compiler.codegen_backend().link(sess, codegen_results, &outputs);
                abort_on_err(result, sess);
            } else {
                sess.fatal("rlink must be a file")
            }
            Compilation::Stop
        } else {
            Compilation::Continue
        }
    }

    pub fn list_metadata(
        sess: &Session,
        metadata_loader: &dyn MetadataLoader,
        input: &Input,
    ) -> Compilation {
        if sess.opts.debugging_opts.ls {
            match *input {
                Input::File(ref ifile) => {
                    let path = &(*ifile);
                    let mut v = Vec::new();
                    locator::list_file_metadata(&sess.target, path, metadata_loader, &mut v)
                        .unwrap();
                    println!("{}", String::from_utf8(v).unwrap());
                }
                Input::Str { .. } => {
                    early_error(ErrorOutputType::default(), "cannot list metadata for stdin");
                }
            }
            return Compilation::Stop;
        }

        Compilation::Continue
    }

    fn print_crate_info(
        codegen_backend: &dyn CodegenBackend,
        sess: &Session,
        input: Option<&Input>,
        odir: &Option<PathBuf>,
        ofile: &Option<PathBuf>,
        temps_dir: &Option<PathBuf>,
    ) -> Compilation {
        use rustc_session::config::PrintRequest::*;
        // PrintRequest::NativeStaticLibs is special - printed during linking
        // (empty iterator returns true)
        if sess.opts.prints.iter().all(|&p| p == PrintRequest::NativeStaticLibs) {
            return Compilation::Continue;
        }

        let attrs = match input {
            None => None,
            Some(input) => {
                let result = parse_crate_attrs(sess, input);
                match result {
                    Ok(attrs) => Some(attrs),
                    Err(mut parse_error) => {
                        parse_error.emit();
                        return Compilation::Stop;
                    }
                }
            }
        };
        for req in &sess.opts.prints {
            match *req {
                TargetList => {
                    let mut targets =
                        rustc_target::spec::TARGETS.iter().copied().collect::<Vec<_>>();
                    targets.sort_unstable();
                    println!("{}", targets.join("\n"));
                }
                Sysroot => println!("{}", sess.sysroot.display()),
                TargetLibdir => println!("{}", sess.target_tlib_path.dir.display()),
                TargetSpec => println!("{}", sess.target.to_json().pretty()),
                FileNames | CrateName => {
                    let input = input.unwrap_or_else(|| {
                        early_error(ErrorOutputType::default(), "no input file provided")
                    });
                    let attrs = attrs.as_ref().unwrap();
                    let t_outputs = rustc_interface::util::build_output_filenames(
                        input, odir, ofile, temps_dir, attrs, sess,
                    );
                    let id = rustc_session::output::find_crate_name(sess, attrs, input);
                    if *req == PrintRequest::CrateName {
                        println!("{}", id);
                        continue;
                    }
                    let crate_types = collect_crate_types(sess, attrs);
                    for &style in &crate_types {
                        let fname =
                            rustc_session::output::filename_for_input(sess, style, &id, &t_outputs);
                        println!("{}", fname.file_name().unwrap().to_string_lossy());
                    }
                }
                Cfg => {
                    let mut cfgs = sess
                        .parse_sess
                        .config
                        .iter()
                        .filter_map(|&(name, value)| {
                            // Note that crt-static is a specially recognized cfg
                            // directive that's printed out here as part of
                            // rust-lang/rust#37406, but in general the
                            // `target_feature` cfg is gated under
                            // rust-lang/rust#29717. For now this is just
                            // specifically allowing the crt-static cfg and that's
                            // it, this is intended to get into Cargo and then go
                            // through to build scripts.
                            if (name != sym::target_feature || value != Some(sym::crt_dash_static))
                                && !sess.is_nightly_build()
                                && find_gated_cfg(|cfg_sym| cfg_sym == name).is_some()
                            {
                                return None;
                            }

                            if let Some(value) = value {
                                Some(format!("{}=\"{}\"", name, value))
                            } else {
                                Some(name.to_string())
                            }
                        })
                        .collect::<Vec<String>>();

                    cfgs.sort();
                    for cfg in cfgs {
                        println!("{}", cfg);
                    }
                }
                RelocationModels
                | CodeModels
                | TlsModels
                | TargetCPUs
                | StackProtectorStrategies
                | TargetFeatures => {
                    codegen_backend.print(*req, sess);
                }
                // Any output here interferes with Cargo's parsing of other printed output
                PrintRequest::NativeStaticLibs => {}
            }
        }
        Compilation::Stop
    }
}

/// Prints version information
pub fn version(binary: &str, matches: &getopts::Matches) {
    let verbose = matches.opt_present("verbose");

    println!("{} {}", binary, util::version_str().unwrap_or("unknown version"));

    if verbose {
        fn unw(x: Option<&str>) -> &str {
            x.unwrap_or("unknown")
        }
        println!("binary: {}", binary);
        println!("commit-hash: {}", unw(util::commit_hash_str()));
        println!("commit-date: {}", unw(util::commit_date_str()));
        println!("host: {}", config::host_triple());
        println!("release: {}", unw(util::release_str()));

        let debug_flags = matches.opt_strs("Z");
        let backend_name = debug_flags.iter().find_map(|x| x.strip_prefix("codegen-backend="));
        get_codegen_backend(&None, backend_name).print_version();
    }
}

fn usage(verbose: bool, include_unstable_options: bool, nightly_build: bool) {
    let groups = if verbose { config::rustc_optgroups() } else { config::rustc_short_optgroups() };
    let mut options = getopts::Options::new();
    for option in groups.iter().filter(|x| include_unstable_options || x.is_stable()) {
        (option.apply)(&mut options);
    }
    let message = "Usage: rustc [OPTIONS] INPUT";
    let nightly_help = if nightly_build {
        "\n    -Z help             Print unstable compiler options"
    } else {
        ""
    };
    let verbose_help = if verbose {
        ""
    } else {
        "\n    --help -v           Print the full set of options rustc accepts"
    };
    let at_path = if verbose {
        "    @path               Read newline separated options from `path`\n"
    } else {
        ""
    };
    println!(
        "{options}{at_path}\nAdditional help:
    -C help             Print codegen options
    -W help             \
              Print 'lint' options and default settings{nightly}{verbose}\n",
        options = options.usage(message),
        at_path = at_path,
        nightly = nightly_help,
        verbose = verbose_help
    );
}

fn print_wall_help() {
    println!(
        "
The flag `-Wall` does not exist in `rustc`. Most useful lints are enabled by
default. Use `rustc -W help` to see all available lints. It's more common to put
warning settings in the crate root using `#![warn(LINT_NAME)]` instead of using
the command line flag directly.
"
    );
}

/// Write to stdout lint command options, together with a list of all available lints
pub fn describe_lints(sess: &Session, lint_store: &LintStore, loaded_plugins: bool) {
    println!(
        "
Available lint options:
    -W <foo>           Warn about <foo>
    -A <foo>           \
              Allow <foo>
    -D <foo>           Deny <foo>
    -F <foo>           Forbid <foo> \
              (deny <foo> and all attempts to override)

"
    );

    fn sort_lints(sess: &Session, mut lints: Vec<&'static Lint>) -> Vec<&'static Lint> {
        // The sort doesn't case-fold but it's doubtful we care.
        lints.sort_by_cached_key(|x: &&Lint| (x.default_level(sess.edition()), x.name));
        lints
    }

    fn sort_lint_groups(
        lints: Vec<(&'static str, Vec<LintId>, bool)>,
    ) -> Vec<(&'static str, Vec<LintId>)> {
        let mut lints: Vec<_> = lints.into_iter().map(|(x, y, _)| (x, y)).collect();
        lints.sort_by_key(|l| l.0);
        lints
    }

    let (plugin, builtin): (Vec<_>, _) =
        lint_store.get_lints().iter().cloned().partition(|&lint| lint.is_plugin);
    let plugin = sort_lints(sess, plugin);
    let builtin = sort_lints(sess, builtin);

    let (plugin_groups, builtin_groups): (Vec<_>, _) =
        lint_store.get_lint_groups().iter().cloned().partition(|&(.., p)| p);
    let plugin_groups = sort_lint_groups(plugin_groups);
    let builtin_groups = sort_lint_groups(builtin_groups);

    let max_name_len =
        plugin.iter().chain(&builtin).map(|&s| s.name.chars().count()).max().unwrap_or(0);
    let padded = |x: &str| {
        let mut s = " ".repeat(max_name_len - x.chars().count());
        s.push_str(x);
        s
    };

    println!("Lint checks provided by rustc:\n");
    println!("    {}  {:7.7}  {}", padded("name"), "default", "meaning");
    println!("    {}  {:7.7}  {}", padded("----"), "-------", "-------");

    let print_lints = |lints: Vec<&Lint>| {
        for lint in lints {
            let name = lint.name_lower().replace('_', "-");
            println!(
                "    {}  {:7.7}  {}",
                padded(&name),
                lint.default_level(sess.edition()).as_str(),
                lint.desc
            );
        }
        println!("\n");
    };

    print_lints(builtin);

    let max_name_len = max(
        "warnings".len(),
        plugin_groups
            .iter()
            .chain(&builtin_groups)
            .map(|&(s, _)| s.chars().count())
            .max()
            .unwrap_or(0),
    );

    let padded = |x: &str| {
        let mut s = " ".repeat(max_name_len - x.chars().count());
        s.push_str(x);
        s
    };

    println!("Lint groups provided by rustc:\n");
    println!("    {}  sub-lints", padded("name"));
    println!("    {}  ---------", padded("----"));
    println!("    {}  all lints that are set to issue warnings", padded("warnings"));

    let print_lint_groups = |lints: Vec<(&'static str, Vec<LintId>)>| {
        for (name, to) in lints {
            let name = name.to_lowercase().replace('_', "-");
            let desc = to
                .into_iter()
                .map(|x| x.to_string().replace('_', "-"))
                .collect::<Vec<String>>()
                .join(", ");
            println!("    {}  {}", padded(&name), desc);
        }
        println!("\n");
    };

    print_lint_groups(builtin_groups);

    match (loaded_plugins, plugin.len(), plugin_groups.len()) {
        (false, 0, _) | (false, _, 0) => {
            println!("Lint tools like Clippy can provide additional lints and lint groups.");
        }
        (false, ..) => panic!("didn't load lint plugins but got them anyway!"),
        (true, 0, 0) => println!("This crate does not load any lint plugins or lint groups."),
        (true, l, g) => {
            if l > 0 {
                println!("Lint checks provided by plugins loaded by this crate:\n");
                print_lints(plugin);
            }
            if g > 0 {
                println!("Lint groups provided by plugins loaded by this crate:\n");
                print_lint_groups(plugin_groups);
            }
        }
    }
}

fn describe_debug_flags() {
    println!("\nAvailable options:\n");
    print_flag_list("-Z", config::DB_OPTIONS);
}

fn describe_codegen_flags() {
    println!("\nAvailable codegen options:\n");
    print_flag_list("-C", config::CG_OPTIONS);
}

fn print_flag_list<T>(
    cmdline_opt: &str,
    flag_list: &[(&'static str, T, &'static str, &'static str)],
) {
    let max_len = flag_list.iter().map(|&(name, _, _, _)| name.chars().count()).max().unwrap_or(0);

    for &(name, _, _, desc) in flag_list {
        println!(
            "    {} {:>width$}=val -- {}",
            cmdline_opt,
            name.replace('_', "-"),
            desc,
            width = max_len
        );
    }
}

/// Process command line options. Emits messages as appropriate. If compilation
/// should continue, returns a getopts::Matches object parsed from args,
/// otherwise returns `None`.
///
/// The compiler's handling of options is a little complicated as it ties into
/// our stability story. The current intention of each compiler option is to
/// have one of two modes:
///
/// 1. An option is stable and can be used everywhere.
/// 2. An option is unstable, and can only be used on nightly.
///
/// Like unstable library and language features, however, unstable options have
/// always required a form of "opt in" to indicate that you're using them. This
/// provides the easy ability to scan a code base to check to see if anything
/// unstable is being used. Currently, this "opt in" is the `-Z` "zed" flag.
///
/// All options behind `-Z` are considered unstable by default. Other top-level
/// options can also be considered unstable, and they were unlocked through the
/// `-Z unstable-options` flag. Note that `-Z` remains to be the root of
/// instability in both cases, though.
///
/// So with all that in mind, the comments below have some more detail about the
/// contortions done here to get things to work out correctly.
pub fn handle_options(args: &[String]) -> Option<getopts::Matches> {
    // Throw away the first argument, the name of the binary
    let args = &args[1..];

    if args.is_empty() {
        // user did not write `-v` nor `-Z unstable-options`, so do not
        // include that extra information.
        let nightly_build =
            rustc_feature::UnstableFeatures::from_environment(None).is_nightly_build();
        usage(false, false, nightly_build);
        return None;
    }

    // Parse with *all* options defined in the compiler, we don't worry about
    // option stability here we just want to parse as much as possible.
    let mut options = getopts::Options::new();
    for option in config::rustc_optgroups() {
        (option.apply)(&mut options);
    }
    let matches = options.parse(args).unwrap_or_else(|e| {
        let msg = match e {
            getopts::Fail::UnrecognizedOption(ref opt) => CG_OPTIONS
                .iter()
                .map(|&(name, ..)| ('C', name))
                .chain(DB_OPTIONS.iter().map(|&(name, ..)| ('Z', name)))
                .find(|&(_, name)| *opt == name.replace('_', "-"))
                .map(|(flag, _)| format!("{}. Did you mean `-{} {}`?", e, flag, opt)),
            _ => None,
        };
        early_error(ErrorOutputType::default(), &msg.unwrap_or_else(|| e.to_string()));
    });

    // For all options we just parsed, we check a few aspects:
    //
    // * If the option is stable, we're all good
    // * If the option wasn't passed, we're all good
    // * If `-Z unstable-options` wasn't passed (and we're not a -Z option
    //   ourselves), then we require the `-Z unstable-options` flag to unlock
    //   this option that was passed.
    // * If we're a nightly compiler, then unstable options are now unlocked, so
    //   we're good to go.
    // * Otherwise, if we're an unstable option then we generate an error
    //   (unstable option being used on stable)
    nightly_options::check_nightly_options(&matches, &config::rustc_optgroups());

    if matches.opt_present("h") || matches.opt_present("help") {
        // Only show unstable options in --help if we accept unstable options.
        let unstable_enabled = nightly_options::is_unstable_enabled(&matches);
        let nightly_build = nightly_options::match_is_nightly_build(&matches);
        usage(matches.opt_present("verbose"), unstable_enabled, nightly_build);
        return None;
    }

    // Handle the special case of -Wall.
    let wall = matches.opt_strs("W");
    if wall.iter().any(|x| *x == "all") {
        print_wall_help();
        rustc_errors::FatalError.raise();
    }

    // Don't handle -W help here, because we might first load plugins.
    let debug_flags = matches.opt_strs("Z");
    if debug_flags.iter().any(|x| *x == "help") {
        describe_debug_flags();
        return None;
    }

    let cg_flags = matches.opt_strs("C");

    if cg_flags.iter().any(|x| *x == "help") {
        describe_codegen_flags();
        return None;
    }

    if cg_flags.iter().any(|x| *x == "no-stack-check") {
        early_warn(
            ErrorOutputType::default(),
            "the --no-stack-check flag is deprecated and does nothing",
        );
    }

    if cg_flags.iter().any(|x| *x == "passes=list") {
        let backend_name = debug_flags.iter().find_map(|x| {
            if x.starts_with("codegen-backend=") {
                Some(&x["codegen-backends=".len()..])
            } else {
                None
            }
        });
        get_codegen_backend(&None, backend_name).print_passes();
        return None;
    }

    if matches.opt_present("version") {
        version("rustc", &matches);
        return None;
    }

    Some(matches)
}

fn parse_crate_attrs<'a>(sess: &'a Session, input: &Input) -> PResult<'a, Vec<ast::Attribute>> {
    match input {
        Input::File(ifile) => rustc_parse::parse_crate_attrs_from_file(ifile, &sess.parse_sess),
        Input::Str { name, input } => rustc_parse::parse_crate_attrs_from_source_str(
            name.clone(),
            input.clone(),
            &sess.parse_sess,
        ),
    }
}

/// Gets a list of extra command-line flags provided by the user, as strings.
///
/// This function is used during ICEs to show more information useful for
/// debugging, since some ICEs only happens with non-default compiler flags
/// (and the users don't always report them).
fn extra_compiler_flags() -> Option<(Vec<String>, bool)> {
    let args = env::args_os().map(|arg| arg.to_string_lossy().to_string()).collect::<Vec<_>>();

    // Avoid printing help because of empty args. This can suggest the compiler
    // itself is not the program root (consider RLS).
    if args.len() < 2 {
        return None;
    }

    let matches = handle_options(&args)?;
    let mut result = Vec::new();
    let mut excluded_cargo_defaults = false;
    for flag in ICE_REPORT_COMPILER_FLAGS {
        let prefix = if flag.len() == 1 { "-" } else { "--" };

        for content in &matches.opt_strs(flag) {
            // Split always returns the first element
            let name = if let Some(first) = content.split('=').next() { first } else { &content };

            let content =
                if ICE_REPORT_COMPILER_FLAGS_STRIP_VALUE.contains(&name) { name } else { content };

            if !ICE_REPORT_COMPILER_FLAGS_EXCLUDE.contains(&name) {
                result.push(format!("{}{} {}", prefix, flag, content));
            } else {
                excluded_cargo_defaults = true;
            }
        }
    }

    if !result.is_empty() { Some((result, excluded_cargo_defaults)) } else { None }
}

/// Runs a closure and catches unwinds triggered by fatal errors.
///
/// The compiler currently unwinds with a special sentinel value to abort
/// compilation on fatal errors. This function catches that sentinel and turns
/// the panic into a `Result` instead.
pub fn catch_fatal_errors<F: FnOnce() -> R, R>(f: F) -> Result<R, ErrorReported> {
    catch_unwind(panic::AssertUnwindSafe(f)).map_err(|value| {
        if value.is::<rustc_errors::FatalErrorMarker>() {
            ErrorReported
        } else {
            panic::resume_unwind(value);
        }
    })
}

/// Variant of `catch_fatal_errors` for the `interface::Result` return type
/// that also computes the exit code.
pub fn catch_with_exit_code(f: impl FnOnce() -> interface::Result<()>) -> i32 {
    let result = catch_fatal_errors(f).and_then(|result| result);
    match result {
        Ok(()) => EXIT_SUCCESS,
        Err(_) => EXIT_FAILURE,
    }
}

static DEFAULT_HOOK: SyncLazy<Box<dyn Fn(&panic::PanicInfo<'_>) + Sync + Send + 'static>> =
    SyncLazy::new(|| {
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|info| {
            // Invoke the default handler, which prints the actual panic message and optionally a backtrace
            (*DEFAULT_HOOK)(info);

            // Separate the output with an empty line
            eprintln!();

            // Print the ICE message
            report_ice(info, BUG_REPORT_URL);
        }));
        hook
    });

/// Prints the ICE message, including query stack, but without backtrace.
///
/// The message will point the user at `bug_report_url` to report the ICE.
///
/// When `install_ice_hook` is called, this function will be called as the panic
/// hook.
pub fn report_ice(info: &panic::PanicInfo<'_>, bug_report_url: &str) {
    let emitter = Box::new(rustc_errors::emitter::EmitterWriter::stderr(
        rustc_errors::ColorConfig::Auto,
        None,
        false,
        false,
        None,
        false,
    ));
    let handler = rustc_errors::Handler::with_emitter(true, None, emitter);

    // a .span_bug or .bug call has already printed what
    // it wants to print.
    if !info.payload().is::<rustc_errors::ExplicitBug>() {
        let d = rustc_errors::Diagnostic::new(rustc_errors::Level::Bug, "unexpected panic");
        handler.emit_diagnostic(&d);
    }

    let mut xs: Vec<Cow<'static, str>> = vec![
        "the compiler unexpectedly panicked. this is a bug.".into(),
        format!("we would appreciate a bug report: {}", bug_report_url).into(),
        format!(
            "rustc {} running on {}",
            util::version_str().unwrap_or("unknown_version"),
            config::host_triple()
        )
        .into(),
    ];

    if let Some((flags, excluded_cargo_defaults)) = extra_compiler_flags() {
        xs.push(format!("compiler flags: {}", flags.join(" ")).into());

        if excluded_cargo_defaults {
            xs.push("some of the compiler flags provided by cargo are hidden".into());
        }
    }

    for note in &xs {
        handler.note_without_error(note);
    }

    // If backtraces are enabled, also print the query stack
    let backtrace = env::var_os("RUST_BACKTRACE").map_or(false, |x| &x != "0");

    let num_frames = if backtrace { None } else { Some(2) };

    interface::try_print_query_stack(&handler, num_frames);

    #[cfg(windows)]
    unsafe {
        if env::var("RUSTC_BREAK_ON_ICE").is_ok() {
            // Trigger a debugger if we crashed during bootstrap
            winapi::um::debugapi::DebugBreak();
        }
    }
}

/// Installs a panic hook that will print the ICE message on unexpected panics.
///
/// A custom rustc driver can skip calling this to set up a custom ICE hook.
pub fn install_ice_hook() {
    SyncLazy::force(&DEFAULT_HOOK);
}

/// This allows tools to enable rust logging without having to magically match rustc's
/// tracing crate version.
pub fn init_rustc_env_logger() {
    if let Err(error) = rustc_log::init_rustc_env_logger() {
        early_error(ErrorOutputType::default(), &error.to_string());
    }
}

/// This allows tools to enable rust logging without having to magically match rustc's
/// tracing crate version. In contrast to `init_rustc_env_logger` it allows you to choose an env var
/// other than `RUSTC_LOG`.
pub fn init_env_logger(env: &str) {
    if let Err(error) = rustc_log::init_env_logger(env) {
        early_error(ErrorOutputType::default(), &error.to_string());
    }
}

#[cfg(all(unix, any(target_env = "gnu", target_os = "macos")))]
mod signal_handler {
    extern "C" {
        fn backtrace_symbols_fd(
            buffer: *const *mut libc::c_void,
            size: libc::c_int,
            fd: libc::c_int,
        );
    }

    extern "C" fn print_stack_trace(_: libc::c_int) {
        const MAX_FRAMES: usize = 256;
        static mut STACK_TRACE: [*mut libc::c_void; MAX_FRAMES] =
            [std::ptr::null_mut(); MAX_FRAMES];
        unsafe {
            let depth = libc::backtrace(STACK_TRACE.as_mut_ptr(), MAX_FRAMES as i32);
            if depth == 0 {
                return;
            }
            backtrace_symbols_fd(STACK_TRACE.as_ptr(), depth, 2);
        }
    }

    // When an error signal (such as SIGABRT or SIGSEGV) is delivered to the
    // process, print a stack trace and then exit.
    pub(super) fn install() {
        unsafe {
            const ALT_STACK_SIZE: usize = libc::MINSIGSTKSZ + 64 * 1024;
            let mut alt_stack: libc::stack_t = std::mem::zeroed();
            alt_stack.ss_sp =
                std::alloc::alloc(std::alloc::Layout::from_size_align(ALT_STACK_SIZE, 1).unwrap())
                    as *mut libc::c_void;
            alt_stack.ss_size = ALT_STACK_SIZE;
            libc::sigaltstack(&alt_stack, std::ptr::null_mut());

            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = print_stack_trace as libc::sighandler_t;
            sa.sa_flags = libc::SA_NODEFER | libc::SA_RESETHAND | libc::SA_ONSTACK;
            libc::sigemptyset(&mut sa.sa_mask);
            libc::sigaction(libc::SIGSEGV, &sa, std::ptr::null_mut());
        }
    }
}

#[cfg(not(all(unix, any(target_env = "gnu", target_os = "macos"))))]
mod signal_handler {
    pub(super) fn install() {}
}

pub fn main() -> ! {
    let start_time = Instant::now();
    let start_rss = get_resident_set_size();
    init_rustc_env_logger();
    signal_handler::install();
    let mut callbacks = TimePassesCallbacks::default();
    install_ice_hook();
    let exit_code = catch_with_exit_code(|| {
        let args = env::args_os()
            .enumerate()
            .map(|(i, arg)| {
                arg.into_string().unwrap_or_else(|arg| {
                    early_error(
                        ErrorOutputType::default(),
                        &format!("argument {} is not valid Unicode: {:?}", i, arg),
                    )
                })
            })
            .collect::<Vec<_>>();
        RunCompiler::new(&args, &mut callbacks).run()
    });

    if callbacks.time_passes {
        let end_rss = get_resident_set_size();
        print_time_passes_entry("total", start_time.elapsed(), start_rss, end_rss);
    }

    process::exit(exit_code)
}
