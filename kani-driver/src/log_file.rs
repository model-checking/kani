// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Serialized appends to the `--log-file` destination.
//!
//! Harnesses run in parallel (see [`crate::harness_runner`]), and both the
//! per-harness summaries and the per-property CBMC output are written from
//! whichever rayon worker produced them. Two properties keep a line intact:
//!
//! 1. One `write` call per line, enforced by [`write_line`]. The line is
//!    assembled in memory, newline included, then handed to a single
//!    [`Write::write_all`]. Formatting into the [`std::fs::File`] instead would
//!    not do: `File` is unbuffered, so `writeln!(file, "{content}")` issues a
//!    separate write per format fragment, letting another thread's line land
//!    between a line and its own newline.
//! 2. A process-wide lock, so the append is serialized rather than relying on
//!    `O_APPEND` atomicity, which holds for the offset update on POSIX but is
//!    not guaranteed for an arbitrarily large write or on every platform Kani
//!    supports.
//!
//! The lock is process-wide rather than per-path because a run has a single
//! `--log-file`; serializing the rare case of two paths costs nothing.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

static LOG_FILE_LOCK: Mutex<()> = Mutex::new(());

/// Append `content` and a newline to `path`, creating the file if needed.
/// Serialized against every other caller.
pub(crate) fn append_line(path: &Path, content: &str) -> std::io::Result<()> {
    // Recover from poisoning rather than propagating: a panic while holding this
    // guard could only have come from the file operations below, which leave no
    // shared state behind, and propagating would turn one failed log write into a
    // panic on every subsequent one.
    let _guard = LOG_FILE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    write_line(&mut file, content)
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

#[cfg(test)]
mod tests {
    use super::{append_line, write_line};
    use std::io::Write;

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
}
