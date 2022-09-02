#![cfg(feature = "unsound_experiments")]
use std::ffi::OsString;
use structopt::StructOpt;
#[derive(Debug, StructOpt)]
pub struct UnsoundExperimentArgs {
    /// Generate C file equivalent to inputted program.
    /// This feature is unstable and it requires `--enable-unstable` to be used
    #[structopt(long, hidden_short_help(true), requires("enable-unstable"))]
    pub unsound_experiment_zero_init_vars: bool,
}

impl UnsoundExperimentArgs {
    pub fn process_args(&self) -> Vec<OsString> {
        let mut flags = vec![];
        if self.unsound_experiment_zero_init_vars {
            flags.push("--zero-init-vars".into());
        }
        flags
    }
}
