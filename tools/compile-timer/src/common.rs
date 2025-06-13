// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct AggrResult {
    pub krate: PathBuf,
    pub krate_trimmed_path: String,
    /// the stats for only the 25th-75th percentile of runs on this crate
    pub iqr_stats: Stats,
    // the stats for all runs on this crate
    full_stats: Stats,
}

fn krate_trimmed_path(krate: &Path) -> String {
    format!("{:?}", krate.strip_prefix(std::env::current_dir().unwrap().parent().unwrap()).unwrap())
}

#[allow(dead_code)]
// Allowing dead code here bc neither of the binaries are named `main.rs`, so the lints
// don't seem to count that we use them there...
impl AggrResult {
    pub fn new(krate: PathBuf, iqr_stats: Stats, full_stats: Stats) -> Self {
        AggrResult { krate_trimmed_path: krate_trimmed_path(&krate), krate, iqr_stats, full_stats }
    }

    pub fn full_std_dev(&self) -> Duration {
        self.full_stats.std_dev
    }

    pub fn iqr(&self) -> Duration {
        self.iqr_stats.range.1 - self.iqr_stats.range.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stats {
    pub avg: Duration,
    pub std_dev: Duration,
    pub range: (Duration, Duration),
}

#[allow(dead_code)]
// Sum the IQR averages and IQR standard deviations respectively for all crates timed.
pub fn aggregate_aggregates(info: &[AggrResult]) -> (Duration, Duration) {
    for i in info {
        println!("krate {:?} -- {:?}", i.krate, i.iqr_stats.avg);
    }

    (info.iter().map(|i| i.iqr_stats.avg).sum(), info.iter().map(|i| i.iqr_stats.std_dev).sum())
}

#[allow(dead_code)]
pub fn fraction_of_duration(dur: Duration, frac: f64) -> Duration {
    Duration::from_nanos(((dur.as_nanos() as f64) * frac) as u64)
}
