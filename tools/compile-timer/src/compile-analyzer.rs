// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![allow(dead_code)]
use crate::common::{AggrResult, fraction_of_duration};
use clap::Parser;
use serde_json::Deserializer;
use std::{cmp::max, fs::File, io, path::PathBuf, time::Duration};
mod common;

// Constants for detecting 'significant' regressions

/// The fractional of a sample's standard deviation that it can regress by
/// without being considered a significant regression.
const FRAC_STD_DEV_THRESHOLD: f64 = 2.0; // In this case, 2x the average std deviation.

/// The fractional amount a run can regress by without it being considered a significant regression.
const FRAC_ABSOLUTE_THRESHOLD: f64 = 0.05; // In this case, 5% of the initial time.

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct AnalyzerArgs {
    #[arg(short, long, value_name = "FILE")]
    path_pre: PathBuf,

    #[arg(short, long, value_name = "FILE")]
    path_post: PathBuf,

    #[arg(short, long)]
    /// The test suite name to display as part of the output's title
    suite_name: Option<String>,

    /// Output results in markdown format
    #[arg(short, long)]
    only_markdown: bool,
}

fn main() {
    let args = AnalyzerArgs::parse();

    let (pre_file, post_file) = try_read_files(&args).unwrap();

    let (pre_ser, post_ser) =
        (Deserializer::from_reader(pre_file), Deserializer::from_reader(post_file));

    let pre_results = pre_ser.into_iter::<AggrResult>().collect::<Vec<_>>();
    let post_results = post_ser.into_iter::<AggrResult>().collect::<Vec<_>>();

    let mut results = pre_results
        .into_iter()
        .filter_map(Result::ok)
        .zip(post_results.into_iter().filter_map(Result::ok))
        .collect::<Vec<_>>();

    sort_results(&mut results);

    if args.only_markdown {
        print_markdown(results.as_slice(), args.suite_name);
    } else {
        print_to_terminal(results.as_slice());
    }
}

/// Sort results based on percentage change, with high magnitude regressions first, then low
/// magnitude regressions, low magnitude improvements and finally high magnitude improvements.
fn sort_results(results: &mut [(AggrResult, AggrResult)]) {
    results.sort_by_key(|a| {
        -(signed_percent_diff(&a.0.iqr_stats.avg, &a.1.iqr_stats.avg).abs() * 1000_f64) as i64
    });
}

/// Print results in a markdown format (for GitHub actions).
fn print_markdown(results: &[(AggrResult, AggrResult)], suite_name: Option<String>) {
    let suite_text = if let Some(suite_name) = suite_name {
        format!(" (`{suite_name}` suite)")
    } else {
        "".to_string()
    };
    println!("# Compiletime Results{suite_text}");
    let total_pre = results.iter().map(|i| i.0.iqr_stats.avg).sum();
    let total_post = results.iter().map(|i| i.1.iqr_stats.avg).sum();
    println!(
        "### *on the whole: {:.2?} → {:.2?} —* {}",
        total_pre,
        total_post,
        diff_string(total_pre, total_post)
    );
    // Note that we have to call the fourth column "heterogeneousness" because the color-formatted
    // diff will cut off if the column isn't wide enough for it, so verbosity is required.
    println!(
        "| test crate | old compile time | new compile time | heterogeneousness (percentage difference) | verdict |"
    );
    println!("| - | - | - | - | - |");
    let regressions = results
        .iter()
        .map(|(pre_res, post_res)| {
            assert!(pre_res.krate_trimmed_path == post_res.krate_trimmed_path);
            let pre_time = pre_res.iqr_stats.avg;
            let post_time = post_res.iqr_stats.avg;

            let verdict = verdict_on_change(pre_res, post_res);
            // emphasize output of crate name if it had a suspected regression
            let emph = if verdict.is_regression() { "**" } else { "" };
            println!(
                "| {emph}{}{emph} | {:.2?} | {:.2?} | {} | {:?} |",
                pre_res.krate_trimmed_path,
                pre_time,
                post_time,
                diff_string(pre_time, post_time),
                verdict
            );
            (&pre_res.krate_trimmed_path, verdict)
        })
        .filter_map(
            |(krate, verdict)| if verdict.is_regression() { Some(krate.clone()) } else { None },
        )
        .collect::<Vec<_>>();

    let footnote_number = 1;
    println!(
        "\n[^{footnote_number}]: threshold: max({FRAC_STD_DEV_THRESHOLD} x std_dev, {FRAC_ABSOLUTE_THRESHOLD} x initial_time)."
    );

    if regressions.is_empty() {
        println!("## No suspected regressions[^{footnote_number}]!");
    } else {
        println!(
            "## Failing because of {} suspected regressions[^{footnote_number}]:",
            regressions.len()
        );
        println!("{}", regressions.join(", "));
        std::process::exit(1);
    }
}

/// Print results for a terminal output.
fn print_to_terminal(results: &[(AggrResult, AggrResult)]) {
    let krate_column_len = results
        .iter()
        .map(|(a, b)| max(a.krate_trimmed_path.len(), b.krate_trimmed_path.len()))
        .max()
        .unwrap();

    for (pre_res, post_res) in results {
        assert!(pre_res.krate == post_res.krate);
        let pre_time = pre_res.iqr_stats.avg;
        let post_time = post_res.iqr_stats.avg;

        let change_dir = if post_time > pre_time {
            "↑"
        } else if post_time == pre_time {
            "-"
        } else {
            "↓"
        };
        let change_amount = (pre_time.abs_diff(post_time).as_micros() as f64
            / post_time.as_micros() as f64)
            * 100_f64;

        println!(
            "krate {:krate_column_len$} -- [{:.2?} => {:.2?} ({change_dir}{change_amount:5.2}%)] {:?}",
            pre_res.krate_trimmed_path,
            pre_time,
            post_time,
            verdict_on_change(pre_res, post_res)
        );
    }
}

/// Classify a change into a [Verdict], determining whether it was an improvement, regression,
/// or likely just noise based on provided thresholds.
fn verdict_on_change(pre: &AggrResult, post: &AggrResult) -> Verdict {
    let (pre_time, post_time) = (pre.iqr_stats.avg, post.iqr_stats.avg);

    if post_time.abs_diff(pre_time) < fraction_of_duration(pre_time, FRAC_ABSOLUTE_THRESHOLD) {
        return Verdict::ProbablyNoise(NoiseExplanation::SmallPercentageChange);
    }

    let avg_std_dev = (pre.full_std_dev() + post.full_std_dev()) / 2;
    if post_time.abs_diff(pre_time) < fraction_of_duration(avg_std_dev, FRAC_STD_DEV_THRESHOLD) {
        return Verdict::ProbablyNoise(NoiseExplanation::SmallComparedToStdDevOf(avg_std_dev));
    }

    if pre.iqr_stats.avg > post.iqr_stats.avg {
        return Verdict::Improved;
    }

    Verdict::PotentialRegression { sample_std_dev: avg_std_dev }
}

fn signed_percent_diff(pre: &Duration, post: &Duration) -> f64 {
    let change_amount = (pre.abs_diff(*post).as_micros() as f64 / pre.as_micros() as f64) * 100_f64;
    if post < pre { -change_amount } else { change_amount }
}

fn diff_string(pre: Duration, post: Duration) -> String {
    let change_dir = if post > pre {
        "$\\color{red}\\textsf{↑ "
    } else if post == pre {
        "$\\color{black}\\textsf{- "
    } else {
        "$\\color{green}\\textsf{↓ "
    };
    let change_amount = signed_percent_diff(&pre, &post).abs();
    format!("{change_dir}{:.2?} ({change_amount:.2}\\\\%)}}$", pre.abs_diff(post))
}

#[derive(Debug)]
enum Verdict {
    /// This crate now compiles faster!
    Improved,
    /// This crate compiled slower, but likely because of OS noise.
    ProbablyNoise(NoiseExplanation),
    /// This crate compiled slower, potentially indicating a true performance regression.
    PotentialRegression { sample_std_dev: std::time::Duration },
}

#[derive(Debug)]
/// The reason a regression was flagged as likely noise rather than a true performance regression.
enum NoiseExplanation {
    /// The increase in compile time is so small compared to the
    /// sample's standard deviation (< [FRAC_STD_DEV_THRESHOLD] * std_dev)
    /// that it is probably just sampling noise.
    SmallComparedToStdDevOf(std::time::Duration),
    /// The percentage increase in compile time is so small (< [FRAC_ABSOLUTE_THRESHOLD]),
    /// the difference is likely insignificant.
    SmallPercentageChange,
}

impl Verdict {
    fn is_regression(&self) -> bool {
        matches!(self, Verdict::PotentialRegression { sample_std_dev: _ })
    }
}

fn try_read_files(c: &AnalyzerArgs) -> io::Result<(File, File)> {
    io::Result::Ok((
        File::open(c.path_pre.canonicalize()?)?,
        File::open(c.path_post.canonicalize()?)?,
    ))
}
