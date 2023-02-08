// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(feature = "unsound_experiments")]
use clap::{Arg, ArgAction, ArgMatches, Command};
use kani_queries::{QueryDb, UserInput};
/// Option used for zero initilizing variables.
const ZERO_INIT_VARS: &str = "unsound-experiment-zero-init-vars";

pub fn add_unsound_experiments_to_parser(app: Command) -> Command {
    app.arg(
        Arg::new(ZERO_INIT_VARS)
            .long(ZERO_INIT_VARS)
            .help("POTENTIALLY UNSOUND EXPERIMENTAL FEATURE. Zero initialize variables")
            .action(ArgAction::SetTrue),
    )
}

pub fn add_unsound_experiment_args_to_queries(queries: &mut QueryDb, matches: &ArgMatches) {
    let mut experiments = queries.get_unsound_experiments();
    experiments.zero_init_vars = matches.get_flag(ZERO_INIT_VARS);
    queries.set_unsound_experiments(experiments);
}
