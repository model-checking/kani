// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module used to configure a compiler session.

use crate::parser;
use clap::ArgMatches;
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

// Custom panic hook.
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
            eprintln!(
                "If you are seeing this message, please file an issue here: {}",
                BUG_REPORT_URL
            );
        }));
        hook
    });

/// Initialize compiler session.
pub fn init_session(args: &ArgMatches) {
    // Initialize the rustc logger using value from RUSTC_LOG. We keep the log control separate
    // because we cannot control the RUSTC log format unless if we match the exact tracing
    // version used by RUSTC.
    rustc_driver::init_rustc_env_logger();

    // Kani panic hook.
    init_panic_hook();

    // Kani logger initialization.
    init_logger(args);
}

/// Initialize the logger using the KANI_LOG environment variable and the --log-level argument.
fn init_logger(args: &ArgMatches) {
    let filter = EnvFilter::from_env(LOG_ENV_VAR);
    let filter = if let Some(log_level) = args.value_of(parser::LOG_LEVEL) {
        filter.add_directive(Directive::from_str(log_level).unwrap())
    } else {
        filter
    };

    if args.is_present(parser::JSON_OUTPUT) {
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
    let use_colors = atty::is(atty::Stream::Stdout) || args.is_present(parser::COLOR_OUTPUT);
    let subscriber = Registry::default().with(filter);
    let subscriber = subscriber.with(
        HierarchicalLayer::default()
            .with_writer(std::io::stdout)
            .with_indent_lines(true)
            .with_ansi(use_colors)
            .with_targets(true)
            .with_verbose_exit(true)
            .with_indent_amount(4),
    );
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

fn init_panic_hook() {
    // Install panic hook
    LazyLock::force(&PANIC_HOOK); // Install ice hook
}
