// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module used to configure a compiler session.

use crate::parser;
use clap::ArgMatches;
use rustc_errors::{
    emitter::Emitter, emitter::HumanReadableErrorType, fallback_fluent_bundle, json::JsonEmitter,
    ColorConfig, Diagnostic, TerminalUrl, DEFAULT_LOCALE_RESOURCE,
};
use std::panic;
use std::str::FromStr;
use std::sync::LazyLock;
use tracing_subscriber::{filter::Directive, layer::SubscriberExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

/// Environment variable used to control this session log tracing.
const LOG_ENV_VAR: &str = "KANI_LOG";

// Include Kani's bug reporting URL in our panics.
const BUG_REPORT_URL: &str =
    "https://github.com/model-checking/kani/issues/new?labels=bug&template=bug_report.md";

// Custom panic hook when running under user friendly message format.
#[allow(clippy::type_complexity)]
static PANIC_HOOK: LazyLock<Box<dyn Fn(&panic::PanicInfo<'_>) + Sync + Send + 'static>> =
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
static JSON_PANIC_HOOK: LazyLock<Box<dyn Fn(&panic::PanicInfo<'_>) + Sync + Send + 'static>> =
    LazyLock::new(|| {
        let hook = panic::take_hook();
        panic::set_hook(Box::new(|info| {
            // Print stack trace.
            let msg = format!("Kani unexpectedly panicked at {info}.",);
            let fallback_bundle = fallback_fluent_bundle(vec![DEFAULT_LOCALE_RESOURCE], false);
            let mut emitter = JsonEmitter::basic(
                false,
                HumanReadableErrorType::Default(ColorConfig::Never),
                None,
                fallback_bundle,
                None,
                false,
                false,
                TerminalUrl::No,
            );
            let diagnostic = Diagnostic::new(rustc_errors::Level::Bug, msg);
            emitter.emit_diagnostic(&diagnostic);
            (*JSON_PANIC_HOOK)(info);
        }));
        hook
    });

/// Initialize compiler session.
pub fn init_session(args: &ArgMatches, json_hook: bool) {
    // Initialize the rustc logger using value from RUSTC_LOG. We keep the log control separate
    // because we cannot control the RUSTC log format unless if we match the exact tracing
    // version used by RUSTC.
    // TODO: Enable rustc log when we upgrade the toolchain.
    // <https://github.com/model-checking/kani/issues/2283>
    //
    // rustc_driver::init_rustc_env_logger();

    // Install Kani panic hook.
    if json_hook {
        json_panic_hook()
    }

    // Kani logger initialization.
    init_logger(args);
}

/// Initialize the logger using the KANI_LOG environment variable and the --log-level argument.
fn init_logger(args: &ArgMatches) {
    let filter = EnvFilter::from_env(LOG_ENV_VAR);
    let filter = if let Some(log_level) = args.get_one::<String>(parser::LOG_LEVEL) {
        filter.add_directive(Directive::from_str(log_level).unwrap())
    } else {
        filter
    };

    if args.get_flag(parser::JSON_OUTPUT) {
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
fn hier_logs(args: &ArgMatches, filter: EnvFilter) {
    let use_colors = atty::is(atty::Stream::Stdout) || args.get_flag(parser::COLOR_OUTPUT);
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
