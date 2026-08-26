// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module defines all compiler extensions that form the Kani compiler.
//!
//! The [KaniCompiler] can be used across multiple rustc driver runs ([`rustc_driver::run_compiler`]),
//! which is used to implement stubs.
//!
//! In the first run, [KaniCompiler::config] will implement the compiler configuration and it will
//! also collect any stubs that may need to be applied. This method will be a no-op for any
//! subsequent runs. The [KaniCompiler] will parse options that are passed via `-C llvm-args`.
//!
//! If no stubs need to be applied, the compiler will proceed to generate goto code, and it won't
//! need any extra runs. However, if stubs are required, we will have to restart the rustc driver
//! in order to apply the stubs. For the subsequent runs, we add the stub configuration to
//! `-C llvm-args`.

use crate::args::{Arguments, BackendOption};
#[cfg(feature = "llbc")]
use crate::codegen_aeneas_llbc::LlbcCodegenBackend;
#[cfg(feature = "cprover")]
use crate::codegen_cprover_gotoc::GotocCodegenBackend;
use crate::kani_middle::check_crate_items;
use crate::kani_queries::QUERY_DB;
use crate::session::init_session;
use clap::Parser;
use rustc_ast::{ast, attr};
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_driver::{Callbacks, Compilation, run_compiler};
use rustc_interface::Config;
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;
use rustc_parse::lexer::StripTokens;
use rustc_parse::new_parser_from_source_str;
use rustc_parse::parser::{AllowConstBlockItems, ForceCollect};
use rustc_public::rustc_internal;
use rustc_session::config::ErrorOutputType;
use rustc_span::{FileName, sym};
use tracing::debug;

/// rustc flags kani-compiler requires for correct verification semantics, set
/// unconditionally on every kani-mode invocation. kani's MIR analysis and
/// goto-c codegen assume abort-on-panic, checked overflow, v0 mangling,
/// encoded MIR, storage markers, and `cfg(kani)`. `cargo kani` already passes
/// these (`kani_rustc_flags()` in kani-driver); for that path appending is
/// idempotent — see "Override semantics" below. For any other caller — a build
/// system that drives kani-compiler directly, or a contributor running
/// `RUSTC=kani-compiler cargo build` to debug — omitting one of these flags
/// previously produced an incorrect or failed verification: missing
/// `--cfg=kani` is a vacuous 0-harness pass, missing `-Coverflow-checks=on`
/// or `-Zmir-enable-passes` proves a different program (silent), and the
/// rest are hard errors. Now the compiler enforces the correct values.
///
/// Override semantics. These are appended, not prepended, and the three
/// relevant rustc behaviors are not uniform:
/// - Scalar `-C`/`-Z` options (`panic`, `overflow-checks`,
///   `symbol-mangling-version`, `always-encode-mir`, `panic_abort_tests`) are
///   last-flag-wins, so appending makes them non-overridable by the caller.
///   That is the point: the caller cannot weaken a verification invariant.
///   The cost is that a future Kani feature wanting one of these *off* has to
///   change this list, not just kani-driver — compare `--prove-safety-only`,
///   which sets `-Cdebug-assertions=off` in the driver alone.
/// - `-Zmir-enable-passes` ACCUMULATES rather than overriding: each occurrence
///   pushes onto one list. So appending `-RemoveStorageMarkers` does not
///   discard a caller's earlier entry. This matters for `--coverage`, which
///   adds `-Zmir-enable-passes=-SingleUseConsts` in `kani_rustc_flags()`: both
///   disables survive. Were it last-flag-wins, appending here would silently
///   re-enable `SingleUseConsts` under `--coverage`.
/// - `--cfg`/`--check-cfg` are additive.
///
/// One caveat for direct callers: appending `--check-cfg` *enables* cfg
/// checking for a caller that passed none, so that crate's own undeclared cfgs
/// begin emitting `unexpected_cfgs` warnings. Warnings only, never a build
/// failure, and `cargo kani` is unaffected because kani-driver already passes
/// `--check-cfg=cfg(kani)`.
///
/// Single-token `--flag=value` encoding: rustc's getopts-based CLI parses
/// `--cfg=kani` and `--cfg kani` identically, and this is the same encoding
/// kani-driver has always passed (`base_rustc_flags()`). The integration test
/// at `tests/script-based-pre/compiler_defaults/` pins that both cfg flags
/// are parsed and applied in this exact form.
///
/// Deliberately NOT included:
/// - `-Clinker=echo` — would clobber a real linker a build system provides
///   for rlib output (last-flag-wins).
/// - `-Zcrate-attr=feature(register_tool)` / `-Zcrate-attr=register_tool(kanitool)`
///   — `-Zcrate-attr` ERRORS on a duplicate registration. `cargo kani` already
///   passes these via `base_rustc_flags()`; appending again would break every
///   `cargo kani` invocation (`error: tool 'kanitool' was already registered`).
///   Omitting them is a loud compile error (`unrecognized tool kanitool`), not
///   silent unsoundness — caller responsibility is acceptable here.
/// - Conditional flags (`-Cinstrument-coverage`, `-Zno-codegen`,
///   `-Cdebug-assertions=off`, `-Zrandomize-layout`) — encode session intent;
///   the caller chooses them.
/// - Diagnostic-format flags (`-Ztrim-diagnostic-paths`, `-Zhuman_readable_cgu_names`,
///   `-Zunstable-options`) — caller preferences, not invariants.
const KANI_REQUIRED_RUSTC_ARGS: &[&str] = &[
    "-Cpanic=abort",
    "-Coverflow-checks=on",
    "-Csymbol-mangling-version=v0",
    "-Zalways-encode-mir",
    "-Zpanic_abort_tests=yes",
    "-Zmir-enable-passes=-RemoveStorageMarkers",
    "--cfg=kani",
    "--check-cfg=cfg(kani)",
];

/// Run the Kani flavour of the compiler.
/// This may require multiple runs of the rustc driver ([`rustc_driver::run_compiler`]).
pub fn run(mut args: Vec<String>) {
    args.extend(KANI_REQUIRED_RUSTC_ARGS.iter().map(|s| s.to_string()));
    let mut kani_compiler = KaniCompiler::new();
    kani_compiler.run(args);
}

/// Configure the LLBC backend (Aeneas's IR).
#[allow(unused)]
fn llbc_backend(args: Arguments) -> Box<dyn CodegenBackend> {
    #[cfg(feature = "llbc")]
    {
        QUERY_DB.with(|db| db.borrow_mut().set_args(args));
        Box::new(LlbcCodegenBackend::new())
    }
    #[cfg(not(feature = "llbc"))]
    unreachable!()
}

/// Configure the cprover backend that generates goto-programs.
#[allow(unused)]
fn cprover_backend(args: Arguments) -> Box<dyn CodegenBackend> {
    #[cfg(feature = "cprover")]
    {
        QUERY_DB.with(|db| db.borrow_mut().set_args(args));
        Box::new(GotocCodegenBackend::new())
    }
    #[cfg(not(feature = "cprover"))]
    unreachable!()
}

#[cfg(any(feature = "cprover", feature = "llbc"))]
fn backend(args: Arguments) -> Box<dyn CodegenBackend> {
    let backend = args.backend;
    match backend {
        #[cfg(feature = "cprover")]
        BackendOption::CProver => cprover_backend(args),
        #[cfg(feature = "llbc")]
        BackendOption::Llbc => llbc_backend(args),
    }
}

/// Fallback backend. It will trigger an error if no backend has been enabled.
#[cfg(not(any(feature = "cprover", feature = "llbc")))]
fn backend(args: Arguments) -> Box<CodegenBackend> {
    compile_error!("No backend is available. Use `cprover` or `llbc`.");
}

/// This object controls the compiler behavior.
///
/// It is responsible for initializing the query database, as well as controlling the compiler
/// state machine.
struct KaniCompiler {
    /// Whether we are currently building the standard library. When set, Kani's
    /// macro overrides are not injected (the macros are defined in the standard
    /// library being built, and `kani` is not available).
    build_std: bool,
    /// Whether the user requested to skip injecting Kani's macro overrides
    /// (`--no-assert-overrides`).
    no_assert_overrides: bool,
}

impl KaniCompiler {
    /// Create a new [KaniCompiler] instance.
    pub fn new() -> KaniCompiler {
        KaniCompiler { build_std: false, no_assert_overrides: false }
    }

    /// Compile the current crate with the given arguments.
    ///
    /// Since harnesses may have different attributes that affect compilation, Kani compiler can
    /// actually invoke the rust compiler multiple times.
    pub fn run(&mut self, args: Vec<String>) {
        debug!(?args, "run_compilation_session");
        run_compiler(&args, self);
    }
}

/// Use default function implementations.
impl Callbacks for KaniCompiler {
    /// Configure the [KaniCompiler] `self` object during the [CompilationStage::Init].
    fn config(&mut self, config: &mut Config) {
        let mut args = vec!["kani-compiler".to_string()];
        // `kani-driver` passes the `kani-compiler` specific arguments through llvm-args, so extract them here.
        args.extend(config.opts.cg.llvm_args.iter().cloned());
        let args = Arguments::parse_from(args);
        init_session(&args, matches!(config.opts.error_format, ErrorOutputType::Json { .. }));

        // Remember whether we are building the standard library so that
        // `after_crate_root_parsing` can decide whether to inject Kani's macro overrides.
        self.build_std = args.build_std;
        self.no_assert_overrides = args.no_assert_overrides;

        // Capture args in the closure so they're available when the backend is created
        // (potentially on a different thread).
        config.make_codegen_backend = Some(Box::new({
            let args = args.clone();
            move |_sess| backend(args)
        }));
    }

    /// Inject Kani's macro overrides into every (non-`#![no_std]`) crate we compile.
    ///
    /// Kani overrides several `core`/`std` macros (e.g. `assert_eq!` without a
    /// `Debug` bound, `panic!` with Kani-specific message handling, reachability
    /// instrumentation for `assert!`). These overrides are intentionally kept out of
    /// the standard-library prelude: since they are `core`-prelude macros, placing
    /// them there makes `#![no_std]` dependencies that import `std`'s prelude
    /// explicitly (`extern crate std; use std::prelude::v1::*;`, e.g. `lazy_static`)
    /// ambiguous against the auto-injected `core` prelude, which is a hard error
    /// (rust-lang/rust E0659).
    ///
    /// Instead we bring the overrides into scope crate-wide by injecting a
    /// `#[macro_use]`d re-import of the standard library. This is applied to every
    /// crate Kani codegens — dependencies included, so that their assertions get the
    /// same instrumentation and messages as the crate under verification — except:
    ///  - when building the standard library itself (the macros live there and `kani`
    ///    is not available), and
    ///  - `#![no_std]`/`#![no_core]` crates, which must not gain an `extern crate std`
    ///    (and which is exactly the case that would otherwise hit E0659).
    ///
    /// A `#![no_std]` dependency therefore keeps the regular `core` macros. This is
    /// sound: Kani still intercepts the panics they lower to during codegen.
    fn after_crate_root_parsing(
        &mut self,
        compiler: &Compiler,
        krate: &mut ast::Crate,
    ) -> Compilation {
        if !self.build_std
            && !self.no_assert_overrides
            && !attr::contains_name(&krate.attrs, sym::no_std)
            && !attr::contains_name(&krate.attrs, sym::no_core)
            // Only inject when an external `std` is available to import. The
            // `verify-std` flow builds `std` itself (no `--extern std`), so
            // injecting `extern crate std` there would pull in a *second* `std`
            // (and `core`), causing duplicate-lang-item errors (E0152).
            && compiler.sess.opts.externs.get("std").is_some()
            // Do not inject into external (registry/git) dependencies: the
            // `#[macro_use]` scope is ambiguous (E0659) with an explicit glob
            // import of the same macro name, and external crates may
            // legitimately do that (e.g. libc >= 0.2.188 re-exports
            // `core::assert` in an internal prelude that its modules
            // glob-import, and calls `assert!`). External dependencies thus
            // use the real `assert!`/`panic!`/... macros, which Kani models
            // soundly, consistent with `no_std` dependencies which never had
            // the overrides. Cargo passes `--cap-lints allow` exactly for
            // non-local dependencies, so use that as the discriminator (like
            // rustc's `Session::opts` consumers do for dependency-only
            // behavior); local path/workspace crates and standalone `kani`
            // builds keep the overrides.
            && compiler.sess.opts.lint_cap.is_none()
        {
            inject_kani_macro_overrides(compiler, krate);
        }
        Compilation::Continue
    }

    /// After analysis, we check the crate items for Kani API misuse or configuration issues.
    fn after_analysis(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: TyCtxt<'_>,
    ) -> Compilation {
        rustc_internal::run(tcx, || {
            let ignore_global_asm = QUERY_DB.with(|db| db.borrow().args().ignore_global_asm);
            check_crate_items(tcx, ignore_global_asm);
        })
        .unwrap();
        Compilation::Continue
    }
}

/// Inject `#[macro_use] extern crate std as _kani_std_macros;` at the crate root so
/// that Kani's `#[macro_export]`ed overrides (`assert!`, `assert_eq!`, `panic!`, …)
/// shadow the corresponding `std`/`core` prelude macros throughout the crate,
/// including nested modules.
///
/// A `#[macro_use]` extern crate places the macros in the higher-priority
/// "`macro_use` prelude" scope, so unlike a glob `use` (which would conflict with the
/// standard prelude, rust-lang/rust E0659) this shadows the prelude macros without
/// ambiguity. The crate is aliased (`as _kani_std_macros`) to avoid clashing with the
/// compiler-injected `extern crate std`.
fn inject_kani_macro_overrides(compiler: &Compiler, krate: &mut ast::Crate) {
    let psess = &compiler.sess.psess;
    let source = "#[allow(unused_extern_crates, unused_imports)]\n\
                  #[macro_use]\n\
                  extern crate std as _kani_std_macros;\n"
        .to_string();
    let filename = FileName::Custom("kani_macro_overrides".to_string());
    let mut parser = match new_parser_from_source_str(psess, filename, source, StripTokens::Nothing)
    {
        Ok(parser) => parser,
        Err(errors) => {
            // This is an internal, statically-known source string, so a parse error
            // indicates a Kani bug rather than a user error.
            for error in errors {
                error.emit();
            }
            return;
        }
    };
    match parser.parse_item(ForceCollect::No, AllowConstBlockItems::No) {
        Ok(Some(item)) => krate.items.insert(0, item),
        Ok(None) => unreachable!("failed to parse Kani's macro-override injection"),
        Err(error) => {
            error.emit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KANI_REQUIRED_RUSTC_ARGS;

    /// Join the two-token `-C opt=val` / `-Z opt=val` / `--cfg val` encodings into single
    /// logical flags, so the invariants below hold no matter which encoding
    /// `KANI_REQUIRED_RUSTC_ARGS` uses. rustc's getopts CLI accepts both forms
    /// interchangeably, so a future edit could switch encodings without changing behavior;
    /// these assertions must not silently stop applying if it does.
    fn logical_flags() -> Vec<String> {
        normalize_encodings(KANI_REQUIRED_RUSTC_ARGS)
    }

    /// Bare `-C`/`-Z` take their value as the next token and join without a separator
    /// (`-C` + `linker=echo` == `-Clinker=echo`); bare long options join with `=`
    /// (`--cfg` + `kani` == `--cfg=kani`).
    fn normalize_encodings(args: &[&str]) -> Vec<String> {
        let mut flags = Vec::new();
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match *arg {
                "-C" | "-Z" => {
                    let value = args.next().expect("`-C`/`-Z` must be followed by a value");
                    flags.push(format!("{arg}{value}"));
                }
                "--cfg" | "--check-cfg" => {
                    let value = args.next().expect("`--cfg` must be followed by a value");
                    flags.push(format!("{arg}={value}"));
                }
                single_token => flags.push(single_token.to_string()),
            }
        }
        flags
    }

    /// The required-flags list must never include a linker override — a build
    /// system that needs a real linker for rlib output would have it silently
    /// clobbered (last-flag-wins).
    #[test]
    fn required_args_do_not_clobber_linker() {
        assert!(
            !logical_flags().iter().any(|f| f.starts_with("-Clinker")),
            "KANI_REQUIRED_RUSTC_ARGS must not set -Clinker (build systems own that)"
        );
    }

    /// The soundness invariants kani's MIR analysis assumes.
    #[test]
    fn required_args_cover_soundness_invariants() {
        let flags = logical_flags();
        for must_have in
            ["-Cpanic=abort", "-Coverflow-checks=on", "-Zalways-encode-mir", "--cfg=kani"]
        {
            assert!(
                flags.iter().any(|f| f == must_have),
                "missing soundness-critical flag: {must_have}"
            );
        }
    }

    /// `-Zcrate-attr` errors on duplicate registration. `cargo kani` already
    /// passes `-Z crate-attr=...`; including it here would break every
    /// `cargo kani` invocation with `error: tool 'kanitool' was already registered`.
    #[test]
    fn required_args_do_not_duplicate_crate_attr() {
        assert!(
            !logical_flags().iter().any(|f| f.starts_with("-Zcrate-attr")),
            "KANI_REQUIRED_RUSTC_ARGS must not include -Zcrate-attr (cargo kani passes it; rustc errors on duplicate)"
        );
    }

    /// The invariants above are only meaningful if `normalize_encodings` really does join
    /// the two-token forms; a silent regression there would defeat all three.
    #[test]
    fn normalize_encodings_joins_both_forms() {
        assert_eq!(
            normalize_encodings(&[
                "-C",
                "linker=echo",
                "-Z",
                "crate-attr=register_tool(kanitool)",
                "--cfg",
                "kani",
                "-Cpanic=abort",
            ]),
            [
                "-Clinker=echo",
                "-Zcrate-attr=register_tool(kanitool)",
                "--cfg=kani",
                "-Cpanic=abort"
            ]
        );
    }
}
