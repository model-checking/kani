// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Serialized writes to the `--log-file` destination.
//!
//! Harnesses run in parallel (see [`crate::harness_runner`]), and both the
//! per-harness summaries and the per-property CBMC output are written from
//! whichever rayon worker produced them. Three properties matter:
//!
//! 1. One `write` call per line, enforced by [`write_line`]. The line is
//!    assembled in memory, newline included, then handed to a single
//!    [`Write::write_all`]. Formatting into the [`File`] instead would not do:
//!    `File` is unbuffered, so `writeln!(file, "{content}")` issues a separate
//!    write per format fragment, letting another thread's line land between a
//!    line and its own newline.
//! 2. A process-wide lock, so writes are serialized rather than relying on
//!    `O_APPEND` atomicity, which holds for the offset update on POSIX but is
//!    not guaranteed for an arbitrarily large write or on every platform Kani
//!    supports.
//! 3. The file holds one run. It is truncated when first opened and the handle
//!    is then reused, so a re-run replaces the previous log instead of appending
//!    to it. This matches `--output-into-files`, whose per-harness files are
//!    written with `File::create`. Appending would leave a reader grepping a
//!    file that silently mixes runs, where a stale `VERIFICATION:- FAILED` from
//!    an earlier run reads as a result of the current one.
//!
//! A run has a single `--log-file`, so one cached handle suffices; a caller that
//! alternated between two paths would truncate on each switch.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The open log file, with the path it was opened for. `None` until the first write.
static LOG_FILE: Mutex<Option<(PathBuf, File)>> = Mutex::new(None);

/// Write `content` and a newline to `path`, truncating the file on the first
/// write of the run and creating it if needed. Serialized against every other
/// caller.
pub(crate) fn append_line(path: &Path, content: &str) -> std::io::Result<()> {
    // Recover from poisoning rather than propagating: a panic while holding this
    // guard could only have come from the file operations below, which leave no
    // shared state behind, and propagating would turn one failed log write into a
    // panic on every subsequent one.
    let mut open_file = LOG_FILE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    if !matches!(open_file.as_ref(), Some((opened_for, _)) if opened_for == path) {
        let file = OpenOptions::new().create(true).write(true).truncate(true).open(path)?;
        *open_file = Some((path.to_path_buf(), file));
    }
    let (_, file) = open_file.as_mut().expect("log file was just opened");

    write_line(file, content)
}

/// Write `content` plus a newline as a single `write_all`.
///
/// Split out from [`append_line`] so the one-write property is testable against
/// an unbuffered writer without touching the filesystem.
fn write_line(out: &mut impl Write, content: &str) -> std::io::Result<()> {
    let mut line = String::with_capacity(content.len() + 1);
    line.push_str(content);
    line.push('\n');
    out.write_all(line.as_bytes())
}

/// Drop the cached handle, so the next [`append_line`] opens (and truncates) afresh.
/// Only a new process does this in practice; tests use it to act as a second run.
#[cfg(test)]
fn forget_open_file() {
    *LOG_FILE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

#[cfg(test)]
mod tests {
    use super::{append_line, forget_open_file, write_line};
    use std::io::Write;
    use std::sync::Mutex;

    /// `append_line` caches one handle in a process-wide static, so the tests that
    /// go through it must not run concurrently with each other.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    /// Counts the `write` calls made through it.
    struct CountingWriter {
        writes: usize,
        bytes: Vec<u8>,
    }

    impl Write for CountingWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.writes += 1;
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// The property that keeps concurrent lines whole: exactly one write reaches
    /// the unbuffered file, so there is no window for another thread to append
    /// between a line and its newline.
    #[test]
    fn write_line_issues_exactly_one_write() {
        let mut out = CountingWriter { writes: 0, bytes: Vec::new() };
        write_line(&mut out, "some rendered property").unwrap();
        assert_eq!(out.writes, 1, "a line must reach the file as one write");
        assert_eq!(out.bytes, b"some rendered property\n");
    }

    /// The shape this module exists to avoid, kept as a live demonstration that
    /// the single-buffer assembly in `write_line` is what makes the difference:
    /// `writeln!` straight into an unbuffered writer splits the line from its
    /// newline. A future refactor back to `writeln!(file, ..)` should have to
    /// delete this test rather than silently reintroduce the tear.
    #[test]
    fn formatting_into_the_writer_splits_the_line() {
        let mut out = CountingWriter { writes: 0, bytes: Vec::new() };
        // `black_box` keeps this a runtime value: given a literal, rustc flattens
        // `format_args!("{}\n", "lit")` into one piece and no split occurs. The real
        // caller formats a runtime `&str`, which is the shape reproduced here.
        let content = std::hint::black_box(String::from("some rendered property"));
        writeln!(out, "{content}").unwrap();
        assert!(
            out.writes > 1,
            "expected `writeln!` on an unbuffered writer to split the line, got {} write(s)",
            out.writes
        );
    }

    /// End-to-end smoke test over the real file path: every line written from a
    /// separate thread arrives whole, exactly once.
    ///
    /// This exercises the lock but cannot *prove* it necessary — a tear is
    /// timing-dependent and does not reproduce on demand, so this test also
    /// passes against an unlocked implementation. The deterministic guarantee is
    /// `write_line_issues_exactly_one_write` above.
    #[test]
    fn concurrent_appends_arrive_whole() {
        let _one_at_a_time = ONE_AT_A_TIME.lock().unwrap_or_else(|p| p.into_inner());
        forget_open_file();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("concurrent.log");
        let lines: Vec<String> =
            (0..64).map(|i| format!("line-{i:02}-{}", "x".repeat(4096))).collect();

        std::thread::scope(|scope| {
            for line in &lines {
                scope.spawn(|| append_line(&path, line).unwrap());
            }
        });

        let written = std::fs::read_to_string(&path).unwrap();
        let mut observed: Vec<&str> = written.lines().collect();
        observed.sort_unstable();
        let mut expected: Vec<&str> = lines.iter().map(String::as_str).collect();
        expected.sort_unstable();
        assert_eq!(observed, expected, "lines were lost, duplicated, or torn");
    }

    /// Within a run, successive lines accumulate; a new run replaces the file
    /// rather than appending, so a reader never sees two runs spliced together.
    #[test]
    fn a_new_run_replaces_the_previous_log() {
        let _one_at_a_time = ONE_AT_A_TIME.lock().unwrap_or_else(|p| p.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.log");

        forget_open_file(); // first run
        append_line(&path, "first run, line one").unwrap();
        append_line(&path, "first run, line two").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "first run, line one\nfirst run, line two\n",
            "lines within one run must accumulate"
        );

        forget_open_file(); // as a second process would
        append_line(&path, "second run").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "second run\n",
            "a new run must replace the previous log, not append to it"
        );
    }
}
