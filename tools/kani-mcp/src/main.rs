// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A minimal Model Context Protocol (MCP) server for Kani.
//!
//! It speaks newline-delimited JSON-RPC 2.0 over stdio (the MCP stdio transport) and never
//! opens a network socket. Each tool shells out to the user's installed `kani` / `cargo kani`,
//! so the server always uses whatever Kani version the user has; it does not link Kani itself.
//!
//! The surface is deliberately small. Anything not exposed here remains available through the
//! Kani CLI, which the `verify-rust` skill uses as a fallback.

mod process;
mod summary;

use anyhow::{Context, Result, bail};
use process::Output;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// The plugin's compatibility file is the single source of truth for supported Kani versions.
const COMPATIBILITY: &str = include_str!("../../../agent-plugin/compatibility.json");
const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";
pub(crate) const INSTALL_HINT: &str = "cargo install --locked kani-verifier && cargo kani setup";
/// Default wall-clock limit for one tool call that runs Kani.
const DEFAULT_TIMEOUT_S: u64 = 1800;
/// Wall-clock limit for listing, and for the toolchain check of `kani_get_version`.
const SHORT_TIMEOUT_S: u64 = 600;
/// How many lines of Kani output to return when Kani fails before producing results.
const OUTPUT_TAIL_LINES: usize = 60;
/// How many concrete playback tests to return from `kani_get_counterexample`.
const MAX_PLAYBACK_TESTS: usize = 3;

fn main() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle(&request),
            Err(e) => Some(error_response(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(response) = response {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

/// Handle one JSON-RPC message. Notifications (no `id`) get no response.
fn handle(request: &Value) -> Option<Value> {
    let id = request.get("id").cloned()?;
    let method = request.get("method").and_then(Value::as_str).unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => initialize(&params),
        "ping" => json!({}),
        "tools/call" => call_tool(&params),
        "tools/list" => json!({ "tools": tool_definitions() }),
        _ => return Some(error_response(id, -32601, &format!("method not found: {method}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn initialize(params: &Value) -> Value {
    let protocol =
        params.get("protocolVersion").and_then(Value::as_str).unwrap_or(DEFAULT_PROTOCOL_VERSION);
    json!({
        "protocolVersion": protocol,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "kani-mcp", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Structured access to the Kani Rust model checker. These tools cover \
            only common operations; every other Kani feature (autoharness, contracts flags, \
            coverage, solver options, ...) is available by running `cargo kani` directly.",
    })
}

fn tool_definitions() -> Value {
    let path = json!({
        "type": "string",
        "description": "A Cargo project directory (runs `cargo kani`) or a single `.rs` file (runs `kani`)."
    });
    let extra_args = json!({
        "type": "array",
        "items": { "type": "string" },
        "description": "Extra Kani command-line arguments, passed through unchanged (e.g. [\"--default-unwind\", \"8\"]). Crates that use function contracts need [\"-Z\", \"function-contracts\"]."
    });
    let harness = json!({ "type": "string", "description": "Only harnesses whose name contains this string (`--harness`)." });
    let exact = json!({ "type": "boolean", "description": "Match `harness` against the fully qualified name exactly (`--exact`)." });
    let timeout = json!({ "type": "integer", "description": format!("Wall-clock limit in seconds for the whole call (default {DEFAULT_TIMEOUT_S}). On timeout Kani and its children are killed and no results are returned: verify fewer harnesses per call.") });
    let harness_timeout = json!({ "type": "integer", "description": "Per-harness limit in seconds (`--harness-timeout`). A harness that exceeds it is reported as TIMEOUT and the run continues. It does not cover goto-instrument." });
    let export_path = json!({ "type": "string", "description": "Also write Kani's `--export-json` file here, e.g. for the plugin's `evidence.py record`." });
    let log_path = json!({ "type": "string", "description": "Also write Kani's console output here, e.g. for `check-vacuity.py --log` (needs the default output format, so do not combine with `jobs`)." });
    json!([
        {
            "name": "kani_get_version",
            "description": "Report the installed Kani version, whether this plugin supports it, and whether the toolchain works (it verifies a trivial harness, which also checks that CBMC and goto-cc are available).",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "kani_list_harnesses",
            "description": "List the proof harnesses and contracts in a Rust file or Cargo project (`kani list --format json`). By default returns a summary: totals, harness counts per file, and the contracts that have no harness. Use `filter` to list matching harness names, or `detail: \"full\"` for the complete list.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "detail": { "type": "string", "enum": ["summary", "full"], "description": "summary (default) or full." },
                    "filter": { "type": "string", "description": "List the harness and contract names that contain this string." },
                    "extra_args": extra_args
                },
                "required": ["path"]
            }
        },
        {
            "name": "kani_verify_harness",
            "description": "Run Kani and return a compact summary of the results: verdict per harness, failed properties with locations, proof-quality counts (unreachable checks, unsatisfiable covers), checks the run disabled, and the stubs each result relies on. Verifies every harness unless `harness` is given. Kani runs harnesses one at a time unless `jobs` is set.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "harness": harness,
                    "exact": exact,
                    "jobs": { "type": "integer", "description": "Verify this many harnesses in parallel (`--jobs N --output-format terse`). Each job can use several GB of memory." },
                    "timeout_s": timeout,
                    "harness_timeout_s": harness_timeout,
                    "export_path": export_path,
                    "log_path": log_path,
                    "extra_args": extra_args
                },
                "required": ["path"]
            }
        },
        {
            "name": "kani_get_counterexample",
            "description": "Verify one harness with concrete playback and return the generated unit tests: `failure_witnesses` replay a failed check with concrete inputs, `cover_witnesses` show inputs that satisfy a `kani::cover!`. Playback runs the real code, not stubs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "harness": { "type": "string", "description": "The harness to reproduce (`--harness`)." },
                    "exact": exact,
                    "timeout_s": timeout,
                    "harness_timeout_s": harness_timeout,
                    "export_path": export_path,
                    "log_path": log_path,
                    "extra_args": extra_args
                },
                "required": ["path", "harness"]
            }
        }
    ])
}

fn call_tool(params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let token = params.get("_meta").and_then(|m| m.get("progressToken"));
    let result = match name {
        "kani_get_counterexample" => get_counterexample(&args, token),
        "kani_get_version" => get_version(token),
        "kani_list_harnesses" => list_harnesses(&args, token),
        "kani_verify_harness" => verify_harness(&args, token),
        _ => Err(anyhow::anyhow!("unknown tool: {name}")),
    };
    match result {
        Ok(value) => tool_result(value, false),
        Err(e) => tool_result(json!({ "error": format!("{e:#}") }), true),
    }
}

fn tool_result(value: Value, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value).unwrap() }],
        "structuredContent": value,
        "isError": is_error,
    })
}

/// How to invoke Kani on a target: `cargo kani` in a project directory, or `kani <file>` in
/// the file's directory (so reported locations stay relative, as on the command line).
struct Target {
    dir: PathBuf,
    /// The file name, for single-file targets.
    file: Option<String>,
}

impl Target {
    fn from_args(args: &Value) -> Result<Target> {
        let raw = args.get("path").and_then(Value::as_str).context("missing `path` argument")?;
        let path = std::fs::canonicalize(raw).with_context(|| format!("cannot access `{raw}`"))?;
        if path.is_dir() {
            Ok(Target { dir: path, file: None })
        } else if path.extension().is_some_and(|e| e == "rs") {
            let dir = path.parent().unwrap_or(Path::new("/")).to_path_buf();
            let file = path.file_name().map(|f| f.to_string_lossy().into_owned());
            Ok(Target { dir, file })
        } else {
            bail!("`{raw}` is neither a directory nor a `.rs` file")
        }
    }

    /// Build `cargo kani [subcommand] args...` or `kani [subcommand] <file> args...`. The file
    /// goes before `args` because `--cbmc-args` consumes everything after it.
    fn command(&self, subcommand: Option<&str>, args: &[String]) -> Command {
        let mut cmd = match &self.file {
            None => {
                let mut cmd = Command::new("cargo");
                cmd.arg("kani").args(subcommand);
                cmd
            }
            Some(file) => {
                let mut cmd = Command::new("kani");
                cmd.args(subcommand).arg(file);
                cmd
            }
        };
        cmd.args(args).current_dir(&self.dir);
        cmd
    }
}

fn timeout_arg(args: &Value, default: u64) -> Duration {
    Duration::from_secs(args.get("timeout_s").and_then(Value::as_u64).unwrap_or(default))
}

/// The command line of `cmd`, shell-quoted, so that it can be recorded and re-run.
fn command_line(cmd: &Command) -> String {
    std::iter::once(cmd.get_program())
        .chain(cmd.get_args())
        .map(|a| {
            let a = a.to_string_lossy();
            if a.chars().all(|c| c.is_ascii_alphanumeric() || "-_=./:,@+".contains(c)) {
                a.into_owned()
            } else {
                format!("'{}'", a.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn tail(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

fn string_list(args: &Value, key: &str) -> Result<Vec<String>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(vec![]),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| v.as_str().map(String::from).context(format!("`{key}` must contain strings")))
            .collect(),
        Some(_) => bail!("`{key}` must be an array of strings"),
    }
}

fn harness_args(args: &Value) -> Vec<String> {
    let mut out = vec![];
    if let Some(h) = args.get("harness").and_then(Value::as_str) {
        out.extend(["--harness".to_string(), h.to_string()]);
        if args.get("exact").and_then(Value::as_bool).unwrap_or(false) {
            out.push("--exact".into());
        }
    }
    out
}

fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.')).find_map(|word| {
        let mut parts = word.split('.').map(|p| p.parse::<u64>().ok());
        match (parts.next()??, parts.next()??, parts.next()??) {
            (a, b, c) if parts.next().is_none() => Some((a, b, c)),
            _ => None,
        }
    })
}

/// Verify a trivial harness, to check that the whole toolchain (including CBMC and goto-cc)
/// works, not just that `kani --version` answers.
fn toolchain_check(token: Option<&Value>) -> Result<Value> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("check.rs"),
        "#[kani::proof]\nfn kani_mcp_toolchain_check() {}\n",
    )?;
    let mut cmd = Command::new("kani");
    cmd.arg("check.rs").current_dir(dir.path());
    let out = process::run(cmd, Duration::from_secs(SHORT_TIMEOUT_S), token)?;
    let ok = out.exit_code == Some(0) && out.text.contains("VERIFICATION:- SUCCESSFUL");
    let mut result = json!({ "works": ok });
    if !ok {
        result["output_tail"] = json!(tail(&out.text, 20));
        result["hint"] = json!(
            "Kani could not verify a trivial harness. If the output mentions goto-cc or cbmc, \
             run `cargo kani setup` (or, for a development build, put CBMC on PATH)."
        );
    }
    Ok(result)
}

fn get_version(token: Option<&Value>) -> Result<Value> {
    let compat: Value = serde_json::from_str(COMPATIBILITY)?;
    let bound = |key: &str| -> Result<(u64, u64, u64)> {
        compat[key].as_str().and_then(parse_version).with_context(|| format!("bad `{key}`"))
    };
    let (lo, hi) = (bound("min_kani")?, bound("max_kani_exclusive")?);
    let fmt = |v: (u64, u64, u64)| format!("{}.{}.{}", v.0, v.1, v.2);
    let candidates: [(&str, &[&str]); 2] =
        [("cargo kani", &["kani", "--version"]), ("kani", &["--version"])];
    let found = candidates.iter().find_map(|(name, args)| {
        let program = if *name == "kani" { "kani" } else { "cargo" };
        let out = Command::new(program).args(*args).output().ok()?;
        let version = parse_version(&String::from_utf8_lossy(&out.stdout))?;
        out.status.success().then_some((*name, version))
    });
    let (command, installed, compatible, message) = match found {
        None => {
            (None, None, false, format!("Kani was not found. Install it with `{INSTALL_HINT}`."))
        }
        Some((cmd, v)) if v < lo => (
            Some(cmd),
            Some(fmt(v)),
            false,
            format!(
                "Kani {} is older than the minimum supported version {} for this plugin. Upgrade with `{INSTALL_HINT}`.",
                fmt(v),
                fmt(lo)
            ),
        ),
        Some((cmd, v)) if v >= hi => (
            Some(cmd),
            Some(fmt(v)),
            false,
            format!(
                "Kani {} is newer than this plugin supports (< {}). Update the plugin, or install a supported Kani release.",
                fmt(v),
                fmt(hi)
            ),
        ),
        Some((cmd, v)) => (Some(cmd), Some(fmt(v)), true, format!("Kani {} is supported.", fmt(v))),
    };
    let mut result = json!({
        "command": command,
        "compatible": compatible,
        "installed": installed,
        "message": message,
        "plugin_version": compat["plugin_version"],
        "supported_range": format!(">= {}, < {}", fmt(lo), fmt(hi)),
    });
    if compatible {
        let toolchain = toolchain_check(token)?;
        if toolchain["works"] == false {
            result["compatible"] = json!(false);
            result["message"] = json!(format!(
                "Kani {} is supported, but it cannot verify a trivial harness.",
                installed.as_deref().unwrap_or("?")
            ));
        }
        result["toolchain"] = toolchain;
    }
    Ok(result)
}

fn list_harnesses(args: &Value, token: Option<&Value>) -> Result<Value> {
    let target = Target::from_args(args)?;
    // `kani list` writes `kani-list.json` into the working directory. Remove it afterwards
    // unless it was already there, so the tool leaves the project as it found it.
    let list_file = target.dir.join("kani-list.json");
    let existed = list_file.exists();
    let mut list_args = vec!["--format".to_string(), "json".to_string()];
    list_args.extend(string_list(args, "extra_args")?);
    let out = process::run(
        target.command(Some("list"), &list_args),
        timeout_arg(args, SHORT_TIMEOUT_S),
        token,
    )?;
    let contents = std::fs::read_to_string(&list_file);
    if !existed {
        let _ = std::fs::remove_file(&list_file);
    }
    let full: Value = match contents {
        Ok(text) => serde_json::from_str(&text)?,
        Err(_) => bail!(
            "`kani list` did not produce results (exit code {:?}{}):\n{}",
            out.exit_code,
            if out.timed_out { ", timed out" } else { "" },
            tail(&out.text, OUTPUT_TAIL_LINES)
        ),
    };
    let detail = args.get("detail").and_then(Value::as_str).unwrap_or("summary");
    let filter = args.get("filter").and_then(Value::as_str);
    if detail == "full" && filter.is_none() {
        return Ok(full);
    }
    Ok(summarize_list(&full, filter))
}

/// A compact view of `kani list` output: totals, harness counts per file, contracts without
/// a harness, and, with `filter`, the matching harness and contract names.
fn summarize_list(full: &Value, filter: Option<&str>) -> Value {
    let per_file = |key: &str| -> Value {
        let counts: serde_json::Map<String, Value> = full[key]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(file, names)| (file.clone(), json!(names.as_array().map_or(0, Vec::len))))
            .collect();
        Value::Object(counts)
    };
    let contracts = full["contracts"].as_array().cloned().unwrap_or_default();
    let without_harness: Vec<&Value> = contracts
        .iter()
        .filter(|c| c["harnesses"].as_array().is_none_or(Vec::is_empty))
        .map(|c| &c["function"])
        .collect();
    let mut out = json!({
        "kani-version": full["kani-version"],
        "totals": full["totals"],
        "standard_harnesses_per_file": per_file("standard-harnesses"),
        "contract_harnesses_per_file": per_file("contract-harnesses"),
        "contracts_without_harness": without_harness,
    });
    if !without_harness.is_empty() {
        out["note"] = json!(
            "`kani list` can miss the harnesses of contracts on methods; check these by \
             searching for `proof_for_contract` before writing new harnesses."
        );
    }
    if let Some(filter) = filter {
        let names = |key: &str| -> Vec<Value> {
            full[key]
                .as_object()
                .into_iter()
                .flatten()
                .flat_map(|(_, names)| names.as_array().cloned().unwrap_or_default())
                .filter(|n| n.as_str().is_some_and(|n| n.contains(filter)))
                .collect()
        };
        out["matching_standard_harnesses"] = json!(names("standard-harnesses"));
        out["matching_contract_harnesses"] = json!(names("contract-harnesses"));
        out["matching_contracts"] = json!(
            contracts
                .iter()
                .filter(|c| c["function"].as_str().is_some_and(|f| f.contains(filter)))
                .collect::<Vec<_>>()
        );
    }
    out
}

/// Options shared by the tools that run verification.
fn run_options(args: &Value) -> Result<Vec<String>> {
    let mut out = harness_args(args);
    if let Some(n) = args.get("harness_timeout_s").and_then(Value::as_u64) {
        out.extend([
            "-Z".into(),
            "unstable-options".into(),
            "--harness-timeout".into(),
            format!("{n}s"),
        ]);
    }
    out.extend(string_list(args, "extra_args")?);
    Ok(out)
}

/// Run Kani with `--export-json` and return the parsed export together with Kani's console
/// output. The export goes to `export_path` if given, otherwise to a temporary directory;
/// the console output is also written to `log_path` if given.
fn run_with_export(
    target: &Target,
    args: &[String],
    tool_args: &Value,
    token: Option<&Value>,
) -> Result<(Option<Value>, Output, Value)> {
    let dir = tempfile::tempdir()?;
    let export = match tool_args.get("export_path").and_then(Value::as_str) {
        Some(p) => {
            let p = PathBuf::from(p);
            if p.exists() {
                std::fs::remove_file(&p)
                    .with_context(|| format!("cannot replace `{}`", p.display()))?;
            }
            p
        }
        None => dir.path().join("results.json"),
    };
    let mut all_args = vec![
        "-Z".into(),
        "export-json".into(),
        "--export-json".into(),
        export.display().to_string(),
    ];
    all_args.extend_from_slice(args);
    let cmd = target.command(None, &all_args);
    let mut run_info =
        json!({ "command": command_line(&cmd), "cwd": target.dir.display().to_string() });
    let out = process::run(cmd, timeout_arg(tool_args, DEFAULT_TIMEOUT_S), token)?;
    if let Some(log) = tool_args.get("log_path").and_then(Value::as_str) {
        std::fs::write(log, &out.text).with_context(|| format!("cannot write `{log}`"))?;
        run_info["log_path"] = json!(log);
    }
    let doc = match std::fs::read_to_string(&export) {
        Ok(text) => Some(serde_json::from_str(&text).context("malformed Kani JSON export")?),
        Err(_) => None,
    };
    if doc.is_some() && tool_args.get("export_path").is_some() {
        run_info["export_path"] = json!(export.display().to_string());
    }
    Ok((doc, out, run_info))
}

fn no_results_error(out: &Output) -> anyhow::Error {
    if out.timed_out {
        return anyhow::anyhow!(
            "Kani was stopped after {}s (`timeout_s`); Kani writes no results when it is killed. \
             Verify fewer harnesses per call, set `harness_timeout_s`, or reduce input and unwind \
             bounds. Last lines of output:\n{}",
            out.elapsed_s,
            tail(&out.text, OUTPUT_TAIL_LINES)
        );
    }
    anyhow::anyhow!(
        "Kani did not produce verification results (exit code {:?}). This is usually a \
         compilation error or an invalid argument; last lines of output:\n{}",
        out.exit_code,
        tail(&out.text, OUTPUT_TAIL_LINES)
    )
}

fn verify_harness(args: &Value, token: Option<&Value>) -> Result<Value> {
    let target = Target::from_args(args)?;
    let mut kani_args = run_options(args)?;
    if let Some(jobs) = args.get("jobs").and_then(Value::as_u64) {
        kani_args.extend(["--jobs".into(), jobs.to_string(), "--output-format=terse".into()]);
    }
    let (doc, out, run_info) = run_with_export(&target, &kani_args, args, token)?;
    let doc = doc.ok_or_else(|| no_results_error(&out))?;
    let mut result = summary::summarize(&doc, false, summary::DEFAULT_MAX_FAILED);
    result["exit_code"] = json!(out.exit_code);
    result["run"] = run_info;
    Ok(result)
}

/// Extract the unit tests printed by `--concrete-playback=print`.
fn playback_tests(text: &str) -> Vec<String> {
    let mut tests = vec![];
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.starts_with("Concrete playback unit test for ") && lines.next() == Some("```") {
            let body: Vec<&str> = lines.by_ref().take_while(|l| *l != "```").collect();
            tests.push(body.join("\n"));
        }
    }
    tests
}

/// The check a playback test was generated for, from its doc comment
/// (`/// Check for `cover`: "..."`).
fn witness_class(test: &str) -> Option<&str> {
    let line = test.lines().find(|l| l.trim_start().starts_with("/// Check for `"))?;
    let start = line.find('`')? + 1;
    let end = start + line[start..].find('`')?;
    Some(&line[start..end])
}

/// The name of the test function in a playback test.
fn test_name(test: &str) -> Option<&str> {
    let start = test.find("fn ")? + 3;
    let end = start + test[start..].find('(')?;
    Some(test[start..end].trim())
}

fn get_counterexample(args: &Value, token: Option<&Value>) -> Result<Value> {
    let target = Target::from_args(args)?;
    if args.get("harness").and_then(Value::as_str).is_none() {
        bail!("missing `harness` argument");
    }
    let mut kani_args = run_options(args)?;
    kani_args.extend(["-Z", "concrete-playback", "--concrete-playback=print"].map(String::from));
    let (doc, out, run_info) = run_with_export(&target, &kani_args, args, token)?;
    let doc = doc.ok_or_else(|| no_results_error(&out))?;
    let summary = summary::summarize(&doc, true, summary::DEFAULT_MAX_FAILED);
    let (covers, failures): (Vec<String>, Vec<String>) =
        playback_tests(&out.text).into_iter().partition(|t| witness_class(t) == Some("cover"));
    let mut result = json!({ "harnesses": summary["harnesses"], "run": run_info });
    for (key, tests) in [("failure_witnesses", &failures), ("cover_witnesses", &covers)] {
        result[key] = json!(tests.iter().take(MAX_PLAYBACK_TESTS).collect::<Vec<_>>());
        if tests.len() > MAX_PLAYBACK_TESTS {
            result[format!("{key}_omitted")] = json!(
                tests[MAX_PLAYBACK_TESTS..].iter().filter_map(|t| test_name(t)).collect::<Vec<_>>()
            );
        }
    }
    let stubbed = doc["harnesses"].as_array().into_iter().flatten().any(|h| {
        summary::len(&h["attributes"]["stubs"]) > 0
            || summary::len(&h["attributes"]["verified_stubs"]) > 0
    });
    let mut note = if summary["harnesses"].as_array().is_some_and(|h| h.is_empty()) {
        "All selected harnesses passed; there is no counterexample to reproduce.".to_string()
    } else if failures.is_empty() {
        "Kani found failures but generated no failure witness (it needs a failing panic check). \
         Inspect the failed properties instead."
            .to_string()
    } else {
        "Each failure witness replays one failed check with concrete inputs. Add it to the crate \
         with `--concrete-playback=inplace`, then confirm the defect with the plugin's \
         `check-witness.py`, or run it with `cargo kani playback -Z concrete-playback -- <test name>`."
            .to_string()
    };
    if stubbed {
        note.push_str(
            " This harness uses stubs or verified contracts, but playback runs the real code: a \
             witness that passes on the real code points at a stub or a too-weak contract.",
        );
    }
    result["note"] = json!(note);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version("cargo-kani 0.68.0"), Some((0, 68, 0)));
        assert_eq!(parse_version("kani 1.2.3 (cargo plugin)"), Some((1, 2, 3)));
        assert_eq!(parse_version("no version"), None);
    }

    #[test]
    fn classifies_witnesses() {
        let cover = "/// Test generated for harness `h`\n///\n/// Check for `cover`: \"x\"\n#[test]\nfn kani_concrete_playback_h_1() {}";
        let failure =
            "/// Check for `assertion`: \"y\"\n#[test]\nfn kani_concrete_playback_h_2() {}";
        assert_eq!(witness_class(cover), Some("cover"));
        assert_eq!(witness_class(failure), Some("assertion"));
        assert_eq!(test_name(cover), Some("kani_concrete_playback_h_1"));
    }

    #[test]
    fn summarizes_list_output() {
        let full = json!({
            "kani-version": "0.68.0",
            "totals": { "standard-harnesses": 2 },
            "standard-harnesses": { "a.rs": ["check_a", "check_b"] },
            "contract-harnesses": {},
            "contracts": [{ "function": "f", "file": "a.rs", "harnesses": [] }]
        });
        let out = summarize_list(&full, Some("check_a"));
        assert_eq!(out["standard_harnesses_per_file"]["a.rs"], 2);
        assert_eq!(out["contracts_without_harness"], json!(["f"]));
        assert_eq!(out["matching_standard_harnesses"], json!(["check_a"]));
    }

    #[test]
    fn quotes_command_lines() {
        let mut cmd = Command::new("cargo");
        cmd.args(["kani", "--harness", "a b"]);
        assert_eq!(command_line(&cmd), "cargo kani --harness 'a b'");
    }

    #[test]
    fn extracts_playback_tests() {
        let text =
            "noise\nConcrete playback unit test for `h`:\n```\n#[test]\nfn t() {}\n```\nINFO";
        assert_eq!(playback_tests(text), vec!["#[test]\nfn t() {}".to_string()]);
    }

    #[test]
    fn notifications_get_no_response() {
        assert!(
            handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none()
        );
    }

    #[test]
    fn unknown_methods_are_errors() {
        let response = handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "nope" })).unwrap();
        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn lists_tools() {
        let response =
            handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" })).unwrap();
        let names: Vec<&str> = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "kani_get_version",
                "kani_list_harnesses",
                "kani_verify_harness",
                "kani_get_counterexample"
            ]
        );
    }

    #[test]
    fn compatibility_file_is_valid() {
        let compat: Value = serde_json::from_str(COMPATIBILITY).unwrap();
        assert!(parse_version(compat["min_kani"].as_str().unwrap()).is_some());
        assert!(parse_version(compat["max_kani_exclusive"].as_str().unwrap()).is_some());
    }
}
