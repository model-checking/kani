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
use rustc_codegen_ssa::traits::CodegenBackend;
use rustc_driver::{Callbacks, Compilation, run_compiler};
use rustc_interface::Config;
use rustc_middle::ty::TyCtxt;
use rustc_public::rustc_internal;
use rustc_session::config::ErrorOutputType;
use tracing::debug;

/// rustc flags kani-compiler requires for correct verification semantics, set
/// unconditionally on every kani-mode invocation. kani's MIR analysis and
/// goto-c codegen assume abort-on-panic, checked overflow, v0 mangling,
/// encoded MIR, storage markers, and `cfg(kani)`. `cargo kani` already passes
/// these (`kani_rustc_flags()` in kani-driver); for that path appending is
/// idempotent — scalar `-C`/`-Z` flags are last-flag-wins and `--cfg`/
/// `--check-cfg` are additive. For any other caller — a build system that
/// drives kani-compiler directly, or a contributor running
/// `RUSTC=kani-compiler cargo build` to debug — omitting one of these flags
/// previously produced an incorrect or failed verification: missing
/// `--cfg=kani` is a vacuous 0-harness pass, missing `-Coverflow-checks=on`
/// or `-Zmir-enable-passes` proves a different program (silent), and the
/// rest are hard errors. Now the compiler enforces the correct values.
///
/// Appended (not prepended): rustc is last-flag-wins for scalar `-C`/`-Z`
/// options, so appending after the caller's args makes these non-overridable.
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
struct KaniCompiler {}

impl KaniCompiler {
    /// Create a new [KaniCompiler] instance.
    pub fn new() -> KaniCompiler {
        KaniCompiler {}
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

        // Capture args in the closure so they're available when the backend is created
        // (potentially on a different thread).
        config.make_codegen_backend = Some(Box::new({
            let args = args.clone();
            move |_cfg, _| backend(args)
        }));
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

#[cfg(test)]
mod tests {
    use super::KANI_REQUIRED_RUSTC_ARGS;

    /// The required-flags list must never include a linker override — a build
    /// system that needs a real linker for rlib output would have it silently
    /// clobbered (last-flag-wins).
    #[test]
    fn required_args_do_not_clobber_linker() {
        assert!(
            !KANI_REQUIRED_RUSTC_ARGS.iter().any(|f| f.starts_with("-Clinker")),
            "KANI_REQUIRED_RUSTC_ARGS must not set -Clinker (build systems own that)"
        );
    }

    /// The soundness invariants kani's MIR analysis assumes.
    #[test]
    fn required_args_cover_soundness_invariants() {
        for must_have in
            ["-Cpanic=abort", "-Coverflow-checks=on", "-Zalways-encode-mir", "--cfg=kani"]
        {
            assert!(
                KANI_REQUIRED_RUSTC_ARGS.contains(&must_have),
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
            !KANI_REQUIRED_RUSTC_ARGS.iter().any(|f| f.contains("crate-attr")),
            "KANI_REQUIRED_RUSTC_ARGS must not include -Zcrate-attr (cargo kani passes it; rustc errors on duplicate)"
        );
    }
}
