// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(feature = "unsound_experiments")]
use clap::{App, Arg, ArgMatches};
use kani_queries::{QueryDb, UserInput};
/// Option used for zero initilizing variables.
const ZERO_INIT_VARS: &str = "unsound-experiment-zero-init-vars";

pub fn add_unsound_experiments_to_parser<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.arg(
        Arg::with_name(ZERO_INIT_VARS)
            .long(ZERO_INIT_VARS)
            .help("POTENTIALLY UNSOUND EXPERIMENTAL FEATURE. Zero initialize variables"),
    )
}

pub fn add_unsound_experiment_args_to_queries(queries: &mut QueryDb, matches: &ArgMatches) {
    queries.get_unsound_experiments().lock().unwrap().zero_init_vars =
        matches.is_present(ZERO_INIT_VARS);
}
