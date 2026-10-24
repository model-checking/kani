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

pub struct Output {
    pub exit_code: Option<i32>,
    pub text: String,
    pub timed_out: bool,
    pub elapsed_s: u64,
}

/// Run `cmd` to completion, or until `timeout` elapses. `progress_token` is the MCP progress
/// token of the request, if the client sent one.
pub fn run(mut cmd: Command, timeout: Duration, progress_token: Option<&Value>) -> Result<Output> {
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
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if start.elapsed() >= timeout {
            timed_out = true;
            kill_group(&mut child);
            break child.wait().ok();
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
        let out = run(cmd, Duration::from_secs(30), None).unwrap();
        assert_eq!(out.exit_code, Some(3));
        assert!(out.text.contains("out") && out.text.contains("err"));
        assert!(!out.timed_out);
    }

    #[test]
    fn kills_on_timeout() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 30 & sleep 30"]);
        let out = run(cmd, Duration::from_secs(1), None).unwrap();
        assert!(out.timed_out);
        assert!(out.elapsed_s < 10);
    }
}
