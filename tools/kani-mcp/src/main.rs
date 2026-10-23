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

mod summary;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The plugin's compatibility file is the single source of truth for supported Kani versions.
const COMPATIBILITY: &str = include_str!("../../../agent-plugin/compatibility.json");
const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";
const INSTALL_HINT: &str = "cargo install --locked kani-verifier && cargo kani setup";
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
    json!([
        {
            "name": "kani_get_version",
            "description": "Report the installed Kani version and whether this plugin supports it.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "kani_list_harnesses",
            "description": "List the proof harnesses and contracts in a Rust file or Cargo project (`kani list --format json`).",
            "inputSchema": {
                "type": "object",
                "properties": { "path": path, "extra_args": extra_args },
                "required": ["path"]
            }
        },
        {
            "name": "kani_verify_harness",
            "description": "Run Kani and return a compact summary of the results: verdict per harness, failed properties with locations, and proof-quality counts (unreachable checks, unsatisfiable covers). Verifies every harness unless `harness` is given.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "harness": { "type": "string", "description": "Only verify harnesses matching this name (`--harness`)." },
                    "exact": { "type": "boolean", "description": "Match `harness` against the fully qualified name exactly (`--exact`)." },
                    "extra_args": extra_args
                },
                "required": ["path"]
            }
        },
        {
            "name": "kani_get_counterexample",
            "description": "Verify one harness with concrete playback and return the generated unit test(s) that reproduce each failure with concrete input values, plus the failed properties.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "harness": { "type": "string", "description": "The harness to reproduce (`--harness`)." },
                    "exact": { "type": "boolean", "description": "Match `harness` against the fully qualified name exactly (`--exact`)." },
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
    let result = match name {
        "kani_get_counterexample" => get_counterexample(&args),
        "kani_get_version" => get_version(),
        "kani_list_harnesses" => list_harnesses(&args),
        "kani_verify_harness" => verify_harness(&args),
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

struct Output {
    exit_code: Option<i32>,
    text: String,
}

fn run(mut cmd: Command) -> Result<Output> {
    let out = cmd.output().with_context(|| {
        format!("failed to run `{:?}`; is Kani installed? Install it with `{INSTALL_HINT}`", cmd)
    })?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(Output { exit_code: out.status.code(), text })
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

fn get_version() -> Result<Value> {
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
    Ok(json!({
        "command": command,
        "compatible": compatible,
        "installed": installed,
        "message": message,
        "plugin_version": compat["plugin_version"],
        "supported_range": format!(">= {}, < {}", fmt(lo), fmt(hi)),
    }))
}

fn list_harnesses(args: &Value) -> Result<Value> {
    let target = Target::from_args(args)?;
    // `kani list` writes `kani-list.json` into the working directory. Remove it afterwards
    // unless it was already there, so the tool leaves the project as it found it.
    let list_file = target.dir.join("kani-list.json");
    let existed = list_file.exists();
    let mut list_args = vec!["--format".to_string(), "json".to_string()];
    list_args.extend(string_list(args, "extra_args")?);
    let out = run(target.command(Some("list"), &list_args))?;
    let contents = std::fs::read_to_string(&list_file);
    if !existed {
        let _ = std::fs::remove_file(&list_file);
    }
    match contents {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(_) => bail!(
            "`kani list` did not produce results (exit code {:?}):\n{}",
            out.exit_code,
            tail(&out.text, OUTPUT_TAIL_LINES)
        ),
    }
}

/// Run Kani with `--export-json` into a temporary directory and return the parsed export
/// together with Kani's console output.
fn run_with_export(target: &Target, args: &[String]) -> Result<(Option<Value>, Output)> {
    let dir = tempfile::tempdir()?;
    let export = dir.path().join("results.json");
    let mut all_args = vec![
        "-Z".into(),
        "export-json".into(),
        "--export-json".into(),
        export.display().to_string(),
    ];
    all_args.extend_from_slice(args);
    let out = run(target.command(None, &all_args))?;
    let doc = match std::fs::read_to_string(&export) {
        Ok(text) => Some(serde_json::from_str(&text).context("malformed Kani JSON export")?),
        Err(_) => None,
    };
    Ok((doc, out))
}

fn no_results_error(out: &Output) -> anyhow::Error {
    anyhow::anyhow!(
        "Kani did not produce verification results (exit code {:?}). This is usually a \
         compilation error or an invalid argument; last lines of output:\n{}",
        out.exit_code,
        tail(&out.text, OUTPUT_TAIL_LINES)
    )
}

fn verify_harness(args: &Value) -> Result<Value> {
    let target = Target::from_args(args)?;
    let mut kani_args = harness_args(args);
    kani_args.extend(string_list(args, "extra_args")?);
    let (doc, out) = run_with_export(&target, &kani_args)?;
    let doc = doc.ok_or_else(|| no_results_error(&out))?;
    let mut result = summary::summarize(&doc, false, summary::DEFAULT_MAX_FAILED);
    result["exit_code"] = json!(out.exit_code);
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

fn get_counterexample(args: &Value) -> Result<Value> {
    let target = Target::from_args(args)?;
    if args.get("harness").and_then(Value::as_str).is_none() {
        bail!("missing `harness` argument");
    }
    let mut kani_args = harness_args(args);
    kani_args.extend(["-Z", "concrete-playback", "--concrete-playback=print"].map(String::from));
    kani_args.extend(string_list(args, "extra_args")?);
    let (doc, out) = run_with_export(&target, &kani_args)?;
    let doc = doc.ok_or_else(|| no_results_error(&out))?;
    let summary = summary::summarize(&doc, true, summary::DEFAULT_MAX_FAILED);
    let tests = playback_tests(&out.text);
    let mut result = json!({
        "harnesses": summary["harnesses"],
        "playback_tests": tests.iter().take(MAX_PLAYBACK_TESTS).collect::<Vec<_>>(),
    });
    if tests.len() > MAX_PLAYBACK_TESTS {
        result["playback_tests_omitted"] = json!(tests.len() - MAX_PLAYBACK_TESTS);
    }
    result["note"] = json!(if summary["harnesses"].as_array().is_some_and(|h| h.is_empty()) {
        "All selected harnesses passed; there is no counterexample to reproduce."
    } else if tests.is_empty() {
        "Kani found failures but could not generate a concrete playback test (it needs a failing \
         panic check or a satisfiable cover). Inspect the failed properties instead."
    } else {
        "Each test replays one failure with concrete inputs. Add it to the crate with \
         `--concrete-playback=inplace`, then confirm the defect with the plugin's \
         `check-witness.py`, or run it with `cargo kani playback -Z concrete-playback -- <test name>`."
    });
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
