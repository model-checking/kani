// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Running Kani with a timeout and progress notifications.
//!
//! Verification can take hours, so a tool call must never block forever: each run has a wall
//! clock limit, after which Kani and every process it started (CBMC, goto-instrument, ...) are
//! killed. While Kani runs, the server sends MCP progress notifications when the client asked
//! for them with a progress token.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(200);
const PROGRESS_EVERY: Duration = Duration::from_secs(15);
const MEMORY_CHECK_EVERY: Duration = Duration::from_secs(2);

pub struct Output {
    pub exit_code: Option<i32>,
    pub text: String,
    pub timed_out: bool,
    /// Killed because Kani and its children together used more than the memory limit.
    pub memory_exceeded: bool,
    pub elapsed_s: u64,
}

impl Output {
    /// The run was stopped by us (time or memory), so Kani wrote no results.
    pub fn stopped(&self) -> bool {
        self.timed_out || self.memory_exceeded
    }
}

/// Resident memory, in MB, of every process in the process group `pgid` (0 if unknown).
fn group_rss_mb(pgid: u32) -> u64 {
    let Ok(out) = Command::new("ps").args(["-axo", "pgid=,rss="]).output() else { return 0 };
    let total_kb: u64 = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let (group, rss) = (parts.next()?, parts.next()?);
            (group == pgid.to_string()).then(|| rss.parse::<u64>().ok()).flatten()
        })
        .sum();
    total_kb / 1024
}

/// Run `cmd` to completion, or until `timeout` elapses. `progress_token` is the MCP progress
/// token of the request, if the client sent one.
pub fn run(
    mut cmd: Command,
    timeout: Duration,
    progress_token: Option<&Value>,
    memory_limit_mb: Option<u64>,
) -> Result<Output> {
    #[cfg(unix)]
    {
        // A process group of its own, so that a timeout kills Kani's children too.
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    let mut child = cmd.spawn().with_context(|| {
        format!(
            "failed to run `{cmd:?}`; is Kani installed? Install it with `{}`",
            crate::INSTALL_HINT
        )
    })?;
    let buffer = Arc::new(Mutex::new(String::new()));
    let readers: Vec<_> = [
        child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
        child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .map(|mut stream| {
        let buffer = Arc::clone(&buffer);
        thread::spawn(move || {
            let mut chunk = [0u8; 8192];
            while let Ok(n) = stream.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                buffer.lock().unwrap().push_str(&String::from_utf8_lossy(&chunk[..n]));
            }
        })
    })
    .collect();

    let start = Instant::now();
    let mut last_progress = start;
    let mut timed_out = false;
    let mut memory_exceeded = false;
    let mut last_memory_check = start;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if start.elapsed() >= timeout {
            timed_out = true;
            kill_group(&mut child);
            break child.wait().ok();
        }
        if let Some(limit) = memory_limit_mb
            && last_memory_check.elapsed() >= MEMORY_CHECK_EVERY
        {
            last_memory_check = Instant::now();
            if group_rss_mb(child.id()) > limit {
                memory_exceeded = true;
                kill_group(&mut child);
                break child.wait().ok();
            }
        }
        if let Some(token) = progress_token
            && last_progress.elapsed() >= PROGRESS_EVERY
        {
            last_progress = Instant::now();
            let last_line = buffer.lock().unwrap().lines().last().unwrap_or_default().to_string();
            notify_progress(token, start.elapsed().as_secs(), timeout.as_secs(), &last_line);
        }
        thread::sleep(POLL);
    };
    for reader in readers {
        let _ = reader.join();
    }
    let text = std::mem::take(&mut *buffer.lock().unwrap());
    Ok(Output {
        exit_code: status.and_then(|s| s.code()),
        text,
        timed_out,
        memory_exceeded,
        elapsed_s: start.elapsed().as_secs(),
    })
}

fn kill_group(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill").args(["-KILL", &format!("-{}", child.id())]).status();
    }
    let _ = child.kill();
}

fn notify_progress(token: &Value, elapsed: u64, total: u64, last_line: &str) {
    let message =
        format!("Kani running for {elapsed}s (timeout {total}s). Last output: {last_line}");
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": { "progressToken": token, "progress": elapsed, "total": total, "message": message },
    });
    // Stdout is the transport; its lock is reentrant, so this is safe while a request is served.
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{notification}");
    let _ = stdout.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_output_and_exit_code() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo out; echo err >&2; exit 3"]);
        let out = run(cmd, Duration::from_secs(30), None, None).unwrap();
        assert_eq!(out.exit_code, Some(3));
        assert!(out.text.contains("out") && out.text.contains("err"));
        assert!(!out.timed_out);
    }

    #[test]
    fn kills_on_timeout() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 30 & sleep 30"]);
        let out = run(cmd, Duration::from_secs(1), None, None).unwrap();
        assert!(out.timed_out);
        assert!(out.elapsed_s < 10);
    }
}
