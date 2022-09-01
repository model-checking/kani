#![cfg(feature = "unsound_experiments")]

#[derive(Debug, Default)]
pub struct UnsoundExperiments {
    pub zero_init_vars: bool,
}
