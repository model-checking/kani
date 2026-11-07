// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Running Kani with a timeout and progress notifications.
//!
//! Verification can take hours, so a tool call must never block forever: each run has a wall
//! clock limit, after which Kani and every process it started (CBMC, goto-instrument, ...) are
//! killed. While Kani runs, the server sends MCP progress notifications when the client asked
//! for them with a progress token.
//!
//! Every run is also guarded against using too much memory (it mirrors
//! `agent-plugin/scripts/kani_run.py`, which documents it): on Linux with systemd the command runs
//! in its own scope (`systemd-run --user --scope -p MemoryMax=.. -p MemorySwapMax=0 -p OOMPolicy=continue`), so the kernel
//! kills only that run, and the scope's `memory.peak` and `cpu.stat` give its real peak memory and
//! CPU time; elsewhere the process group's resident memory is polled and the group killed above the
//! cap. The cap is `memory_limit_mb`, else `KANI_PLUGIN_MEMORY_MB` (`0` for none), else half of the
//! memory available to this process. `KANI_PLUGIN_MEMORY_GUARD` picks `scope`, `watchdog` or `off`.

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
static RUN_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub struct Output {
    pub exit_code: Option<i32>,
    pub text: String,
    pub timed_out: bool,
    /// Killed because Kani and its children together used more than the memory limit.
    pub memory_exceeded: bool,
    pub elapsed_s: u64,
    /// The largest memory use of the run in MB: the scope's `memory.peak`, or the largest sample
    /// of the watchdog (`None` if unknown).
    pub peak_memory_mb: Option<u64>,
    /// CPU seconds used (known with the scope guard only).
    pub cpu_s: Option<f64>,
    /// `scope`, `watchdog`, or `none`.
    pub guard: &'static str,
    pub limit_mb: Option<u64>,
}

impl Output {
    /// The run was stopped by us (time or memory), so Kani wrote no results.
    pub fn stopped(&self) -> bool {
        self.timed_out || self.memory_exceeded
    }

    /// `memory` or `timeout`, if the run was stopped.
    pub fn killed(&self) -> Option<&'static str> {
        if self.memory_exceeded {
            Some("memory")
        } else if self.timed_out {
            Some("timeout")
        } else {
            None
        }
    }

    /// `killed: memory (peak 9120 MB, limit 8192 MB) after 41s`, if the run was stopped.
    pub fn killed_message(&self) -> Option<String> {
        let reason = self.killed()?;
        let peak = self
            .peak_memory_mb
            .map_or_else(|| "peak unknown".to_string(), |p| format!("peak {p} MB"));
        let limit = match (reason, self.limit_mb) {
            ("memory", Some(l)) => format!(", limit {l} MB"),
            _ => String::new(),
        };
        Some(format!("killed: {reason} ({peak}{limit}) after {}s", self.elapsed_s))
    }

    /// What the run used, as stored in exports and summaries (`plugin_run`).
    pub fn record(&self) -> Value {
        let mut record = json!({
            "elapsed_s": self.elapsed_s, "guard": self.guard, "limit_mb": self.limit_mb,
            "peak_memory_mb": self.peak_memory_mb, "cpu_s": self.cpu_s.map(|c| (c * 10.0).round() / 10.0),
            "exit_code": self.exit_code,
        });
        if let Some(reason) = self.killed() {
            record["killed"] = json!(reason);
            record["killed_message"] = json!(self.killed_message());
        }
        if self.guard == "watchdog" {
            record["peak_is_sampled"] = json!(true);
        }
        record
    }
}

const MIN_DEFAULT_MB: u64 = 2048;

/// The memory this process may use, in MB: the machine's, or its cgroup's limit if smaller.
fn available_memory_mb() -> Option<u64> {
    let physical = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|t| {
            t.lines().find_map(|l| {
                l.strip_prefix("MemTotal:")
                    .and_then(|r| r.split_whitespace().next())
                    .and_then(|kb| kb.parse::<u64>().ok())
                    .map(|kb| kb / 1024)
            })
        })
        .or_else(|| {
            let out = Command::new("sysctl").args(["-n", "hw.memsize"]).output().ok()?;
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse::<u64>()
                .ok()
                .map(|b| b / (1024 * 1024))
        });
    [physical, cgroup_limit_mb()].into_iter().flatten().min()
}

/// The smallest finite `memory.max` on the cgroup path of this process (cgroup v2).
fn cgroup_limit_mb() -> Option<u64> {
    let relative = std::fs::read_to_string("/proc/self/cgroup")
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("0::").map(str::to_string))?;
    let root = std::path::Path::new("/sys/fs/cgroup");
    let path = root.join(relative.trim_start_matches('/'));
    path.ancestors()
        .take_while(|d| d.starts_with(root))
        .filter_map(|d| std::fs::read_to_string(d.join("memory.max")).ok())
        .filter_map(|v| v.trim().parse::<u64>().ok())
        .map(|b| b / (1024 * 1024))
        .min()
}

/// The memory cap in MB, or `None` for no limit: the argument (`0` for none), then
/// `KANI_PLUGIN_MEMORY_MB`, then half of the available memory.
pub fn resolve_limit_mb(explicit: Option<u64>) -> Option<u64> {
    if let Some(n) = explicit {
        return (n > 0).then_some(n);
    }
    match std::env::var("KANI_PLUGIN_MEMORY_MB").ok().as_deref().map(str::trim) {
        Some("") | None => {}
        Some(v) if matches!(v.to_lowercase().as_str(), "0" | "none" | "off" | "unlimited") => {
            return None;
        }
        Some(v) => {
            if let Ok(n) = v.parse::<f64>() {
                return (n >= 1.0).then_some(n as u64);
            }
        }
    }
    available_memory_mb().map(|m| (m / 2).max(MIN_DEFAULT_MB))
}

fn scope_available() -> bool {
    static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OK.get_or_init(|| {
        cfg!(target_os = "linux")
            && Command::new("systemd-run")
                .args(["--user", "--scope", "--quiet", "-p", "MemoryMax=1G", "true"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
    })
}

fn guard_method(limit: Option<u64>, forced: Option<&str>) -> &'static str {
    let from_env = std::env::var("KANI_PLUGIN_MEMORY_GUARD").unwrap_or_default().to_lowercase();
    let forced = forced.map_or(from_env, str::to_string);
    if limit.is_none() || forced == "off" {
        "none"
    } else if forced == "watchdog" || !scope_available() {
        "watchdog"
    } else {
        "scope"
    }
}

/// Runs in the scope after the command: the scope is gone when `systemd-run` returns, so the
/// cgroup's counters are read from inside it.
const SCOPE_WRAPPER: &str = r#""$@"; rc=$?
cg=/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)
{
  echo "peak_bytes=$(cat "$cg/memory.peak" 2>/dev/null)"
  echo "cpu_usec=$(awk '/^usage_usec/{print $2}' "$cg/cpu.stat" 2>/dev/null)"
  echo "oom_kill=$(awk '/^oom_kill /{print $2}' "$cg/memory.events" 2>/dev/null)"
} > "$KANI_PLUGIN_RES" 2>/dev/null
exit $rc"#;

/// `cmd` run inside a systemd scope with a hard memory cap, recording the scope's counters in `res`.
fn scoped(cmd: &Command, limit_mb: u64, res: &std::path::Path, unit: &str) -> Command {
    let mut wrapped = Command::new("systemd-run");
    wrapped
        .args(["--user", "--scope", "--quiet"])
        .arg(format!("--unit={unit}"))
        .arg("-p")
        .arg(format!("MemoryMax={limit_mb}M"))
        .args(["-p", "MemorySwapMax=0", "-p", "OOMPolicy=continue"])
        .args(["--", "bash", "-c", SCOPE_WRAPPER, "_"])
        .arg(cmd.get_program())
        .args(cmd.get_args());
    for (key, value) in cmd.get_envs() {
        match value {
            Some(v) => wrapped.env(key, v),
            None => wrapped.env_remove(key),
        };
    }
    if let Some(dir) = cmd.get_current_dir() {
        wrapped.current_dir(dir);
    }
    wrapped.env("KANI_PLUGIN_RES", res);
    wrapped
}

/// Whether the kernel killed the run for exceeding the scope's cap: it counted an OOM kill, or the
/// run died of SIGKILL or SIGTERM.
fn scope_memory_kill(oom_kills: u64, exit_code: Option<i32>) -> bool {
    // With `OOMPolicy=continue` the wrapper survives; the run died of SIGKILL (137) or SIGTERM (143).
    oom_kills > 0 || matches!(exit_code, Some(137 | 143) | None)
}

/// The counters the scope wrapper wrote: (peak MB, CPU seconds, OOM kills).
fn read_scope_record(res: &std::path::Path) -> (Option<u64>, Option<f64>, u64) {
    let text = std::fs::read_to_string(res).unwrap_or_default();
    let value = |key: &str| -> Option<u64> {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("{key}=")).and_then(|v| v.trim().parse().ok()))
    };
    (
        value("peak_bytes").map(|b| b / (1024 * 1024)),
        value("cpu_usec").map(|u| u as f64 / 1e6),
        value("oom_kill").unwrap_or(0),
    )
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
    cmd: Command,
    timeout: Duration,
    progress_token: Option<&Value>,
    memory_limit_mb: Option<u64>,
) -> Result<Output> {
    run_with_guard(cmd, timeout, progress_token, memory_limit_mb, None)
}

fn run_with_guard(
    mut cmd: Command,
    timeout: Duration,
    progress_token: Option<&Value>,
    memory_limit_mb: Option<u64>,
    forced_guard: Option<&str>,
) -> Result<Output> {
    let limit = resolve_limit_mb(memory_limit_mb);
    let guard = guard_method(limit, forced_guard);
    let res_file = std::env::temp_dir().join(format!(
        "kani-mcp-run-{}-{}.res",
        std::process::id(),
        RUN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let unit = format!(
        "kani-plugin-{}-{}",
        std::process::id(),
        RUN_COUNTER.load(std::sync::atomic::Ordering::Relaxed)
    );
    if let (true, Some(l)) = (guard == "scope", limit) {
        cmd = scoped(&cmd, l, &res_file, &unit);
    }
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
    let mut sampled_peak_mb = 0u64;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if start.elapsed() >= timeout {
            timed_out = true;
            kill_run(&mut child, guard, &unit);
            break child.wait().ok();
        }
        if guard == "watchdog"
            && let Some(limit) = limit
            && last_memory_check.elapsed() >= MEMORY_CHECK_EVERY
        {
            last_memory_check = Instant::now();
            let rss = group_rss_mb(child.id());
            sampled_peak_mb = sampled_peak_mb.max(rss);
            if rss > limit {
                memory_exceeded = true;
                kill_run(&mut child, guard, &unit);
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
    // After a kill, a process that escaped it must not keep us waiting for the pipes to close.
    let deadline = Instant::now()
        + if timed_out || memory_exceeded { Duration::from_secs(3) } else { Duration::MAX / 4 };
    for reader in readers {
        while !reader.is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        if reader.is_finished() {
            let _ = reader.join();
        }
    }
    let text = std::mem::take(&mut *buffer.lock().unwrap());
    let exit_code = status.and_then(|s| s.code());
    let (mut peak_memory_mb, mut cpu_s) = ((sampled_peak_mb > 0).then_some(sampled_peak_mb), None);
    if guard == "scope" {
        let (peak, cpu, oom_kills) = read_scope_record(&res_file);
        let _ = std::fs::remove_file(&res_file);
        (peak_memory_mb, cpu_s) = (peak, cpu);
        memory_exceeded |= !timed_out && scope_memory_kill(oom_kills, exit_code);
    }
    Ok(Output {
        exit_code,
        text,
        timed_out,
        memory_exceeded,
        elapsed_s: start.elapsed().as_secs(),
        peak_memory_mb,
        cpu_s,
        guard,
        limit_mb: limit,
    })
}

/// Kill the run: its scope (every process in it, wherever it went), and its process group.
fn kill_run(child: &mut std::process::Child, guard: &str, unit: &str) {
    if guard == "scope" {
        let _ = Command::new("systemctl")
            .args(["--user", "kill", "--signal=SIGKILL", &format!("{unit}.scope")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    kill_group(child);
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

    const HOG: &str = "import time; x = b'x' * (400 * 1024 * 1024); print('allocated', flush=True); time.sleep(60)";

    #[test]
    fn watchdog_kills_a_run_over_the_cap_alone() {
        let mut cmd = Command::new("python3");
        cmd.args(["-c", HOG]);
        let out = run_with_guard(cmd, Duration::from_secs(60), None, Some(150), Some("watchdog"))
            .unwrap();
        assert!(out.memory_exceeded && !out.timed_out);
        assert!(out.elapsed_s < 30);
        assert!(out.peak_memory_mb.unwrap() > 150);
        let message = out.killed_message().unwrap();
        assert!(message.starts_with("killed: memory (peak "), "{message}");
        assert!(message.contains("limit 150 MB"), "{message}");
        // This process is still here to see it.
        assert_eq!(out.record()["killed"], "memory");
        assert_eq!(out.record()["guard"], "watchdog");
    }

    #[test]
    fn a_run_under_the_cap_reports_its_peak() {
        let mut cmd = Command::new("python3");
        cmd.args(["-c", "import time; x = b'x' * (120 * 1024 * 1024); time.sleep(5)"]);
        let out = run_with_guard(cmd, Duration::from_secs(60), None, Some(4000), Some("watchdog"))
            .unwrap();
        assert!(!out.stopped());
        assert!(out.peak_memory_mb.unwrap() > 50);
    }

    #[test]
    fn limits() {
        assert_eq!(resolve_limit_mb(Some(512)), Some(512));
        assert_eq!(resolve_limit_mb(Some(0)), None);
    }

    #[test]
    fn scope_command_has_a_hard_cap_and_no_swap() {
        let mut cmd = Command::new("kani");
        cmd.arg("x.rs").env("A", "b").current_dir("/tmp");
        let wrapped = scoped(&cmd, 1024, std::path::Path::new("/tmp/res"), "kani-plugin-test");
        let line = format!("{wrapped:?}");
        assert!(line.contains("systemd-run") && line.contains("--scope"), "{line}");
        assert!(line.contains("MemoryMax=1024M") && line.contains("MemorySwapMax=0"), "{line}");
        assert!(line.contains("OOMPolicy=continue"), "{line}");
        assert!(line.contains("--unit=kani-plugin-test"), "{line}");
        assert!(line.contains("\"kani\" \"x.rs\""), "{line}");
        assert!(line.contains("KANI_PLUGIN_RES"), "{line}");
    }

    #[test]
    fn scope_records_and_kill_detection() {
        let res = std::env::temp_dir().join(format!("kani-mcp-res-{}", std::process::id()));
        std::fs::write(&res, "peak_bytes=2147483648\ncpu_usec=4500000\noom_kill=1\n").unwrap();
        let (peak, cpu, kills) = read_scope_record(&res);
        std::fs::remove_file(&res).unwrap();
        assert_eq!((peak, cpu, kills), (Some(2048), Some(4.5), 1));
        assert!(scope_memory_kill(1, Some(0)));
        assert!(scope_memory_kill(0, Some(137)));
        assert!(scope_memory_kill(0, Some(143)));
        assert!(!scope_memory_kill(0, Some(0)));
        assert!(!scope_memory_kill(0, Some(1)));
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
