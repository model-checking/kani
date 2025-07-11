// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module used to configure a compiler session.

use crate::args::Arguments;
use rustc_driver::default_translator;
use rustc_errors::{
    ColorConfig, DiagInner, emitter::Emitter, emitter::HumanReadableErrorType, json::JsonEmitter,
    registry::Registry as ErrorRegistry,
};
use rustc_session::EarlyDiagCtxt;
use rustc_session::config::ErrorOutputType;
use rustc_span::source_map::FilePathMapping;
use rustc_span::source_map::SourceMap;
use std::io;
use std::io::IsTerminal;
use std::panic;
use std::sync::Arc;
use std::sync::LazyLock;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};
use tracing_tree::HierarchicalLayer;

/// Environment variable used to control this session log tracing.
const LOG_ENV_VAR: &str = "KANI_LOG";

// Include Kani's bug reporting URL in our panics.
const BUG_REPORT_URL: &str =
    "https://github.com/model-checking/kani/issues/new?labels=bug&template=bug_report.md";

// Custom panic hook when running under user friendly message format.
#[allow(clippy::type_complexity)]
static PANIC_HOOK: LazyLock<Box<dyn Fn(&panic::PanicHookInfo<'_>) + Sync + Send + 'static>> =
    LazyLock::new(|| {
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|info| {
            // Print stack trace.
            (*PANIC_HOOK)(info);
            eprintln!();

            // Print the Kani message
            eprintln!("Kani unexpectedly panicked during compilation.");
            eprintln!("Please file an issue here: {BUG_REPORT_URL}");
        }));
        hook
    });

// Custom panic hook when executing under json error format `--error-format=json`.
#[allow(clippy::type_complexity)]
static JSON_PANIC_HOOK: LazyLock<Box<dyn Fn(&panic::PanicHookInfo<'_>) + Sync + Send + 'static>> =
    LazyLock::new(|| {
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|info| {
            // Print stack trace.
            let msg = format!("Kani unexpectedly panicked at {info}.",);
            let mut emitter = JsonEmitter::new(
                Box::new(io::BufWriter::new(io::stderr())),
                #[allow(clippy::arc_with_non_send_sync)]
                Some(Arc::new(SourceMap::new(FilePathMapping::empty()))),
                default_translator(),
                false,
                HumanReadableErrorType::Default,
                ColorConfig::Never,
            );
            let registry = ErrorRegistry::new(&[]);
            let diagnostic = DiagInner::new(rustc_errors::Level::Bug, msg);
            emitter.emit_diagnostic(diagnostic, &registry);
            (*JSON_PANIC_HOOK)(info);
        }));
        hook
    });

/// Initialize compiler session.
pub fn init_session(args: &Arguments, json_hook: bool) {
    // Initialize the rustc logger using value from RUSTC_LOG. We keep the log control separate
    // because we cannot control the RUSTC log format unless if we match the exact tracing
    // version used by RUSTC.
    let handler = EarlyDiagCtxt::new(ErrorOutputType::default());
    rustc_driver::init_rustc_env_logger(&handler);

    // Install Kani panic hook.
    if json_hook {
        json_panic_hook()
    }

    // Kani logger initialization.
    init_logger(args);
}

/// Initialize the logger using the KANI_LOG environment variable and the --log-level argument.
fn init_logger(args: &Arguments) {
    let filter = EnvFilter::from_env(LOG_ENV_VAR);
    let filter = if let Some(log_level) = &args.log_level {
        filter.add_directive(log_level.clone())
    } else {
        filter
    };

    if args.json_output {
        json_logs(filter);
    } else {
        hier_logs(args, filter);
    };
}

/// Configure global logger to use a json logger.
fn json_logs(filter: EnvFilter) {
    use tracing_subscriber::fmt::layer;
    let subscriber = Registry::default().with(filter).with(layer().json());
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

/// Configure global logger to use a hierarchical view.
fn hier_logs(args: &Arguments, filter: EnvFilter) {
    let use_colors = std::io::stdout().is_terminal() || args.color_output;
    let subscriber = Registry::default().with(filter);
    let subscriber = subscriber.with(
        HierarchicalLayer::default()
            .with_writer(std::io::stderr)
            .with_indent_lines(true)
            .with_ansi(use_colors)
            .with_targets(true)
            .with_verbose_exit(true)
            .with_indent_amount(4),
    );
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

pub fn init_panic_hook() {
    // Install panic hook
    LazyLock::force(&PANIC_HOOK); // Install ice hook
}

pub fn json_panic_hook() {
    // Install panic hook
    LazyLock::force(&JSON_PANIC_HOOK); // Install ice hook
}
