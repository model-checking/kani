// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Progress indicator for verification harness execution

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Tracks statistics for harness verification progress
#[derive(Debug, Clone)]
pub struct VerificationStats {
    pub total: usize,
    pub completed: Arc<AtomicUsize>,
    pub succeeded: Arc<AtomicUsize>,
    pub failed: Arc<AtomicUsize>,
    pub timed_out: Arc<AtomicUsize>,
}

impl VerificationStats {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            completed: Arc::new(AtomicUsize::new(0)),
            succeeded: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
            timed_out: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn increment_completed(&self) {
        self.completed.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_succeeded(&self) {
        self.succeeded.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_failed(&self) {
        self.failed.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_timed_out(&self) {
        self.timed_out.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_completed(&self) -> usize {
        self.completed.load(Ordering::SeqCst)
    }

    pub fn get_succeeded(&self) -> usize {
        self.succeeded.load(Ordering::SeqCst)
    }

    pub fn get_failed(&self) -> usize {
        self.failed.load(Ordering::SeqCst)
    }

    pub fn get_timed_out(&self) -> usize {
        self.timed_out.load(Ordering::SeqCst)
    }
}

/// Progress indicator for verification harness execution
pub struct ProgressIndicator {
    progress_bar: Option<ProgressBar>,
    stats: VerificationStats,
}

impl ProgressIndicator {
    /// Create a new progress indicator if running in an interactive terminal and log file is enabled
    pub fn new(total_harnesses: usize, show_progress: bool) -> Self {
        let stats = VerificationStats::new(total_harnesses);

        if show_progress {
            let multi_progress = MultiProgress::new();
            let progress_bar = multi_progress.add(ProgressBar::new(total_harnesses as u64));

            progress_bar.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})\n{msg}"
                )
                .unwrap()
                .progress_chars("#>-"),
            );

            progress_bar.set_message("Verifying harnesses...");

            Self { progress_bar: Some(progress_bar), stats }
        } else {
            Self { progress_bar: None, stats }
        }
    }

    /// Update progress with the result of a harness verification
    pub fn update_with_result(&self, succeeded: bool, timed_out: bool) {
        self.stats.increment_completed();

        if timed_out {
            self.stats.increment_timed_out();
        } else if succeeded {
            self.stats.increment_succeeded();
        } else {
            self.stats.increment_failed();
        }

        if let Some(ref pb) = self.progress_bar {
            pb.inc(1);
            let completed = self.stats.get_completed();
            let succeeded = self.stats.get_succeeded();
            let failed = self.stats.get_failed();
            let timed_out = self.stats.get_timed_out();

            pb.set_message(format!(
                "Completed: {}/{} | Succeeded: {} | Failed: {} | Timed out: {}",
                completed, self.stats.total, succeeded, failed, timed_out
            ));
        }
    }

    /// Finish the progress indicator
    pub fn finish(&self) {
        if let Some(ref pb) = self.progress_bar {
            let succeeded = self.stats.get_succeeded();
            let failed = self.stats.get_failed();
            let timed_out = self.stats.get_timed_out();

            pb.finish_with_message(format!(
                "Verification complete | Succeeded: {} | Failed: {} | Timed out: {}",
                succeeded, failed, timed_out
            ));
        }
    }

    /// Check if progress indicator is active
    pub fn is_active(&self) -> bool {
        self.progress_bar.is_some()
    }

    /// Get the statistics
    ///
    /// This method provides access to the underlying verification statistics,
    /// which may be useful for testing, monitoring, or external reporting purposes.
    #[allow(dead_code)]
    pub fn stats(&self) -> &VerificationStats {
        &self.stats
    }
}
