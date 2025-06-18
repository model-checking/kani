// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(exit_status_error)]

use crate::common::{AggrResult, Stats, aggregate_aggregates, krate_trimmed_path};
use clap::Parser;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
mod common;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct TimerArgs {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    out_path: PathBuf,

    /// Ignore compiling the current directory
    #[arg(short, long)]
    skip_current: bool,

    /// The paths of additional paths to visit beyond the current subtree
    #[arg(short, long, value_name = "DIR")]
    also_visit: Vec<PathBuf>,

    /// The names or paths of files to ignore
    #[arg(short, long)]
    ignore: Vec<PathBuf>,
}

/// The number of untimed runs to do before starting timed runs.
/// We need at least one warm-up run to make sure crates are fetched & cached in
/// the local `.cargo/registry` folder. Otherwise the first run will be significantly slower.
const WARMUP_RUNS: usize = 1;
const TIMED_RUNS: usize = 10;

fn main() {
    let args = TimerArgs::parse();

    let (mut to_visit, mut res) = (vec![], vec![]);
    to_visit.extend(args.also_visit.into_iter().rev());
    if !args.skip_current {
        let current_directory = std::env::current_dir().expect("should be run in a directory");
        to_visit.push(current_directory);
    }

    let mut out_ser = serde_json::Serializer::pretty(File::create(&args.out_path).unwrap());
    let run_start = std::time::Instant::now();

    // recursively visit subdirectories to time the compiler on all rust projects
    while let Some(next) = to_visit.pop() {
        let next = next.canonicalize().unwrap();
        let path_to_toml = next.join("Cargo.toml");

        if path_to_toml.exists() && path_to_toml.is_file() {
            // in rust crate so we want to profile it
            println!("[!] profiling in {}", krate_trimmed_path(&next));
            let new_res = profile_on_crate(&next);
            new_res.serialize(&mut out_ser).unwrap();
            res.push(new_res);
        } else {
            // we want want to recur and visit all directories that aren't explicitly ignored
            to_visit.extend(std::fs::read_dir(next).unwrap().filter_map(|entry| {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() && !args.ignore.iter().any(|ignored| path.ends_with(ignored)) {
                        return Some(path);
                    }
                }
                None
            }));
        }
    }

    println!("[!] total info is {:?}", aggregate_aggregates(&res));

    print!("\t [*] run took {:?}", run_start.elapsed());
}

// Profile a crate by running a certain number of untimed warmup runs and then
// a certain number of timed runs, returning aggregates of the timing results.
fn profile_on_crate(absolute_path: &std::path::PathBuf) -> AggrResult {
    let _warmup_results = (0..WARMUP_RUNS)
        .map(|i| {
            print!("\t[*] running warmup {}/{WARMUP_RUNS}", i + 1);
            let _ = std::io::stdout().flush();
            let res = run_command_in(absolute_path);
            println!(" -- {res:?}");
            res
        })
        .collect::<Vec<_>>();

    let timed_results = (0..TIMED_RUNS)
        .map(|i| {
            print!("\t[*] running timed run {}/{TIMED_RUNS}", i + 1);
            let _ = std::io::stdout().flush();
            let res = run_command_in(absolute_path);
            println!(" -- {res:?}");
            res
        })
        .collect::<Vec<_>>();

    let aggr = aggregate_results(absolute_path, &timed_results);
    println!("\t[!] results for {absolute_path:?} are in! {aggr:?}");

    aggr
}

type RunResult = Duration;
// Run `cargo kani` in a crate and parse out the compiler timing info outputted
// by the `TIME_COMPILER` environment variable.
fn run_command_in(absolute_path: &Path) -> RunResult {
    // `cargo clean` first to ensure the compiler is fully run again
    let _ = Command::new("cargo")
        .current_dir(absolute_path)
        .arg("clean")
        .stdout(Stdio::null())
        .output()
        .expect("cargo clean should succeed");

    // do the actual compiler run (using `--only-codegen` to save time)
    let kani_output = Command::new("cargo")
        .current_dir(absolute_path)
        .arg("kani")
        .arg("--only-codegen")
        .env("TIME_COMPILER", "true")
        .output()
        .expect("cargo kani should succeed");

    // parse the output bytes into a string
    let out_str = String::from_utf8(kani_output.stdout).expect("utf8 conversion should succeed");

    if !kani_output.status.success() {
        println!(
            "the `TIME_COMPILER=true cargo kani --only-codegen` command failed in {absolute_path:?} with output -- {out_str:?}"
        );
        panic!("cargo kani command failed");
    }

    // parse that string for the compiler build information
    // and if it's built multiple times (which could happen in a workspace with multiple crates),
    // we just sum up the total time
    out_str.split("\n").filter(|line| line.starts_with("BUILT")).map(extract_duration).sum()
}

fn extract_duration(s: &str) -> Duration {
    let s = &s[s.find("IN").unwrap_or(0)..];
    let micros = s.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().ok().unwrap();

    Duration::from_micros(micros)
}

fn aggregate_results(path: &Path, results: &[Duration]) -> AggrResult {
    assert!(results.len() == TIMED_RUNS);

    // sort and calculate the subset of times in the interquartile range
    let mut sorted = results.to_vec();
    sorted.sort();
    let iqr_bounds = (0.25 * results.len() as f64, 0.75 * results.len() as f64);
    let iqr_durations = sorted
        .into_iter()
        .enumerate()
        .filter_map(|(i, v)| {
            if i >= iqr_bounds.0 as usize && i <= iqr_bounds.1 as usize { Some(v) } else { None }
        })
        .collect::<Vec<Duration>>();

    AggrResult::new(path.to_path_buf(), result_stats(&iqr_durations), result_stats(results))
}

// Record the stats from a subset slice of timing runs.
fn result_stats(results: &[Duration]) -> Stats {
    let avg = results.iter().sum::<Duration>() / results.len().try_into().unwrap();
    let range = (*results.iter().min().unwrap(), *results.iter().max().unwrap());

    let deviations = results.iter().map(|x| x.abs_diff(avg).as_micros().pow(2)).sum::<u128>();
    let std_dev =
        Duration::from_micros((deviations / results.len() as u128).isqrt().try_into().unwrap());

    Stats { avg, std_dev, range }
}
