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

mod autoharness;
mod export;
mod map;
mod partial;
mod process;
mod summary;

use anyhow::{Context, Result, bail};
use process::Output;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
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
        "instructions": "Structured access to the Kani Rust model checker, and to the Crate Verifiability \
            Map (kani_map_crate, kani_plan_verification, kani_update_verification_map, \
            kani_explain_function) for planning what to verify and in which order. These tools cover \
            only common operations; every other Kani feature (contracts flags, coverage, solver \
            options, ...) is available by running `cargo kani` directly.",
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
    let exact = json!({ "type": "boolean", "description": "Match `harness` against the fully qualified name exactly (`--exact`). It then needs the module-qualified name that `kani_list_harnesses` shows (`proofs::check_x`); without `exact`, any substring of the name works." });
    let timeout = json!({ "type": "integer", "description": format!("Wall-clock limit in seconds for the whole call (default {DEFAULT_TIMEOUT_S}). On timeout Kani and its children are killed and no results are returned: verify fewer harnesses per call.") });
    let harness_timeout = json!({ "type": "integer", "description": "Per-harness limit in seconds (`--harness-timeout`). A harness that exceeds it is reported as TIMEOUT and the run continues. It does not cover goto-instrument." });
    let export_path = json!({ "type": "string", "description": "Write Kani's `--export-json` file here (parent directories are created). Default: a file under the plugin output directory (`target/kani-plugin/`, or `KANI_PLUGIN_OUT`), reported as `run.export_path`, for the plugin's `evidence.py record`." });
    let memory_limit = json!({ "type": "integer", "description": "Memory cap in MB (0: none) for Kani and everything it started; a run over it is killed alone (on Linux with systemd, in its own scope), so it cannot take down this server or the machine. Default: `KANI_PLUGIN_MEMORY_MB`, else half of the available memory. A stopped run is reported as `killed: memory` with its peak, and an export with `run_state` KILLED is written (`run.export_path`). Completed runs report `run.resources` (peak memory, CPU)." });
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
                    "per_harness": { "type": "boolean", "description": "Run each harness in its own Kani process (`--harness NAME --exact`) with its own export, and merge the results. Results already produced survive a timeout or a memory kill; `timeout_s` is then the budget of the whole call, and `per_harness_timeout_s` (default 900) the limit of each harness. Use it for sweeps of many harnesses." },
                    "per_harness_timeout_s": { "type": "integer", "description": "With `per_harness`: wall-clock limit of each harness (default 900)." },
                    "memory_limit_mb": memory_limit,
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
            "name": "kani_autoharness",
            "description": "Run Kani's AutoHarness (`kani autoharness`): generate a harness for each eligible function and check it for panics, overflow, and UB on all inputs. With `list_only`, only report which functions get a harness and why the others are skipped. AutoHarness also runs every manual harness in the crate, first, whatever the patterns say, and with a default unwind bound of 20 (kani#5103): results are split into automatic and manual harnesses.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "list_only": { "type": "boolean", "description": "Only list the selected and skipped functions (`--list`); nothing is verified." },
                    "filter": { "type": "string", "description": "With `list_only`: only list function names containing this string." },
                    "include_patterns": { "type": "array", "items": { "type": "string" }, "description": "Only generate harnesses for functions matching these regular expressions (`--include-pattern`). Patterns cannot contain spaces: use `.`, e.g. `Mul<Foo>.for.Bar`." },
                    "exclude_patterns": { "type": "array", "items": { "type": "string" }, "description": "Do not generate harnesses for functions matching these regular expressions (`--exclude-pattern`); takes precedence over include." },
                    "bounded_arguments": { "type": "boolean", "description": "Also cover functions taking slices, `&str`, `Vec`, with bounded inputs (`--bounded-arguments`); results hold only up to the bounds." },
                    "slice_bound": { "type": "integer", "description": "Maximum slice length under `bounded_arguments` (`--slice-bound`)." },
                    "string_bound": { "type": "integer", "description": "Maximum `&str` length in bytes under `bounded_arguments` (`--string-bound`)." },
                    "memory_limit_mb": memory_limit,
                    "failures_only": { "type": "boolean", "description": "Only list harnesses that did not succeed (default true); counts always cover all." },
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
            "name": "kani_map_crate",
            "description": "Build the Crate Verifiability Map of a Cargo workspace: functions, call graph (cargo-callgraph if installed, else a labeled text scan), recursion, existing harnesses and contracts, and the verification status of each function from the Kani results given. Returns a compact summary and the path of the map file; use `kani_plan_verification` and `kani_explain_function` on it. A planning aid, not a proof: unresolved edges, dynamic dispatch, macros, `cfg`, and FFI are listed as uncertainty.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "results": { "type": "array", "items": { "type": "string" }, "description": "`--export-json` files (e.g. `run.export_path` of earlier calls) and evidence bundle directories to merge." },
                    "callgraph": { "type": "string", "enum": ["auto", "tool", "text"], "description": "auto (default): cargo-callgraph if installed; tool: require it; text: text scan only." },
                    "map_path": { "type": "string", "description": "Where to write the map (default: under the plugin output directory)." },
                    "timeout_s": timeout
                },
                "required": ["path"]
            }
        },
        {
            "name": "kani_plan_verification",
            "description": "A staged, bottom-up verification plan from a map: for each function not yet verified, the proposed Kani workflow (contracts for recursion, unwind bounds for loops, stubs for unresolved callees, AutoHarness for simple leaves, ...), its priority, the callees that block it, and what is uncertain. `goals` limit the plan to some functions and everything they depend on.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "map_path": { "type": "string", "description": "A map written by `kani_map_crate`." },
                    "goals": { "type": "array", "items": { "type": "string" }, "description": "Function ids, paths, or names to plan for." },
                    "max_items": { "type": "integer", "description": "How many functions to list (default 40); the rest are counted." },
                    "timeout_s": timeout
                },
                "required": ["map_path"]
            }
        },
        {
            "name": "kani_update_verification_map",
            "description": "Merge new Kani results into a map (`--export-json` files, evidence bundles): statuses, strategies, and priorities are recomputed. Refuses when the sources changed since the map was built: map again.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "map_path": { "type": "string" },
                    "results": { "type": "array", "items": { "type": "string" }, "description": "Export files or evidence bundles to merge." },
                    "root": { "type": "string", "description": "The workspace root (default: the one recorded by `kani_map_crate`)." },
                    "timeout_s": timeout
                },
                "required": ["map_path", "results"]
            }
        },
        {
            "name": "kani_explain_function",
            "description": "Everything a map knows about one function: verification status and evidence, callers and callees with their status, strategy and rationale, priority, and uncertainty. The five complexity metrics are `not_computed` until `cargo kani complexity` exists.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "map_path": { "type": "string" },
                    "function": { "type": "string", "description": "A function id, qualified path, or name." },
                    "timeout_s": timeout
                },
                "required": ["map_path", "function"]
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
                    "memory_limit_mb": memory_limit,
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
        "kani_autoharness" => autoharness_tool(&args, token),
        "kani_map_crate" => map::map_crate(&args, token),
        "kani_plan_verification" => map::plan(&args, token),
        "kani_update_verification_map" => map::update(&args, token),
        "kani_explain_function" => map::explain(&args, token),
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

fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Where the plugin keeps its own files, as `kani_plugin_lib.plugin_out_dir` does: `KANI_PLUGIN_OUT`,
/// else `$CARGO_TARGET_DIR/kani-plugin`, else `<dir>/target/kani-plugin`.
pub(crate) fn plugin_out_dir(dir: &Path) -> PathBuf {
    if let Some(p) = std::env::var_os("KANI_PLUGIN_OUT") {
        return PathBuf::from(p);
    }
    match std::env::var_os("CARGO_TARGET_DIR") {
        Some(t) => PathBuf::from(t).join("kani-plugin"),
        None => dir.join("target").join("kani-plugin"),
    }
}

fn write_creating_dirs(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create `{}`", parent.display()))?;
    }
    std::fs::write(path, text).with_context(|| format!("cannot write `{}`", path.display()))
}

/// The first compiler errors in Kani's output: what to fix when no results were produced.
fn compiler_errors(text: &str) -> Vec<String> {
    let mut errors: Vec<Vec<&str>> = vec![];
    let mut open = false;
    for line in text.lines() {
        if line.starts_with("error:") || line.starts_with("error[") {
            errors.push(vec![line]);
            open = true;
        } else if open
            && (line.starts_with(' ') || line.starts_with("-->"))
            && errors.last().is_some_and(|e| e.len() < 8)
        {
            errors.last_mut().unwrap().push(line);
        } else {
            open = false;
        }
    }
    errors.iter().take(6).map(|e| e.join("\n")).collect()
}

/// Unstable Kani features the crate's sources use (`kani::requires` needs `-Z function-contracts`,
/// `kani::stub` needs `-Z stubbing`, ...), as `-Z feature` arguments, so that the same call works
/// without the caller knowing which flags a crate needs. Features already in `given` are skipped.
fn auto_unstable(dir: &Path, given: &[String]) -> Vec<String> {
    const RULES: &[(&str, &[&str])] = &[
        (
            "function-contracts",
            &[
                "kani::requires",
                "kani::ensures",
                "kani::modifies",
                "proof_for_contract",
                "stub_verified",
            ],
        ),
        ("stubbing", &["kani::stub("]),
        ("loop-contracts", &["loop_invariant", "loop_modifies"]),
    ];
    let mut text = String::new();
    let mut todo = vec![dir.to_path_buf()];
    let mut files = 0;
    while let Some(d) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !name.starts_with('.') && !name.starts_with("target") && name != "node_modules" {
                    todo.push(path);
                }
            } else if name.ends_with(".rs") && files < 5000 {
                files += 1;
                if let Ok(t) = std::fs::read_to_string(&path) {
                    text.push_str(&t);
                }
            }
        }
    }
    let mut out = vec![];
    for (feature, needles) in RULES {
        let already = given.windows(2).any(|w| w[0] == "-Z" && w[1] == *feature);
        if !already && needles.iter().any(|n| text.contains(n)) {
            out.extend(["-Z".to_string(), feature.to_string()]);
        }
    }
    out
}

/// Find `program` on PATH.
fn which(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|p| p.is_file())
}

/// The `channel` of the nearest `rust-toolchain.toml` in an ancestor directory of `path`. Search
/// stops at the first such file (as for rustup); `None` if there is none or it names no channel.
fn toolchain_channel_above(path: &Path) -> Option<String> {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let file =
        real.ancestors().skip(1).map(|d| d.join("rust-toolchain.toml")).find(|f| f.is_file())?;
    std::fs::read_to_string(file).ok()?.lines().find_map(|l| {
        l.trim()
            .strip_prefix("channel")
            .and_then(|r| r.trim_start().strip_prefix('='))
            .map(|v| v.trim().trim_matches(['"', '\'']).to_string())
    })
}

/// The `-Z` option that enables `--export-json` in the installed Kani: `-Z export-json` (builds
/// since the RFC 0015 export) or `-Z unstable-options` (0.68.0 and older builds, which reject the
/// former). Probed once with `kani -Z export-json --version`: Kani validates `-Z` values before it
/// acts on `--version`, so the answer is immediate and nothing is compiled.
/// `KANI_PLUGIN_EXPORT_FLAG` overrides the probe.
fn export_flag_args() -> Vec<String> {
    static FLAG: OnceLock<&'static str> = OnceLock::new();
    let flag = FLAG.get_or_init(|| match std::env::var("KANI_PLUGIN_EXPORT_FLAG").as_deref() {
        Ok("unstable-options") => "unstable-options",
        Ok("export-json") => "export-json",
        _ => probe_export_flag(),
    });
    vec!["-Z".into(), (*flag).into()]
}

fn probe_export_flag() -> &'static str {
    let mut cmd = if which("cargo-kani").is_some() && which("cargo").is_some() {
        let mut cmd = Command::new("cargo");
        cmd.arg("kani");
        cmd
    } else {
        Command::new("kani")
    };
    cmd.args(["-Z", "export-json", "--version"]);
    apply_kani_env(&mut cmd);
    match cmd.output() {
        Ok(out) => export_flag_for(out.status.success(), &String::from_utf8_lossy(&out.stderr)),
        Err(_) => "export-json",
    }
}

/// `-Z export-json` was rejected as an invalid value: this Kani needs `-Z unstable-options`.
fn export_flag_for(accepted: bool, stderr: &str) -> &'static str {
    if !accepted && stderr.contains("invalid value") && stderr.contains("export-json") {
        "unstable-options"
    } else {
        "export-json"
    }
}

/// An installed Rust toolchain to run Kani's `cargo` proxy with: Kani's own toolchain if it
/// can be found, otherwise rustup's default toolchain.
fn kani_toolchain() -> Option<String> {
    if let Some(kani) = which("kani").or_else(|| which("cargo-kani")) {
        // A development build: the binary is `scripts/kani` or `target/kani/bin/kani` somewhere
        // below a Kani checkout, whose `rust-toolchain.toml` is in one of its ancestors.
        if let Some(channel) = toolchain_channel_above(&kani) {
            return Some(channel);
        }
        // A release: `$KANI_HOME/kani-<version>/rust-toolchain-version`.
        let home = std::env::var_os("KANI_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".kani")))?;
        let mut versions: Vec<PathBuf> = std::fs::read_dir(&home)
            .ok()?
            .flatten()
            .map(|e| e.path().join("rust-toolchain-version"))
            .filter(|p| p.is_file())
            .collect();
        versions.sort();
        if let Some(file) = versions.last() {
            return std::fs::read_to_string(file).ok().map(|v| v.trim().to_string());
        }
    }
    let out =
        Command::new("rustup").arg("default").env("RUSTUP_AUTO_INSTALL", "0").output().ok()?;
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().map(String::from)
}

/// rustup's `cargo` proxy reads the project's `rust-toolchain.toml` before Kani starts, and
/// installs that toolchain if it is missing; Kani does not need it (it uses its own). Pin the
/// proxy to an installed toolchain and disable auto-install, unless the user chose one.
fn apply_kani_env(cmd: &mut Command) {
    cmd.env("RUSTUP_AUTO_INSTALL", "0");
    if std::env::var_os("RUSTUP_TOOLCHAIN").is_none()
        && let Some(toolchain) = kani_toolchain()
    {
        cmd.env("RUSTUP_TOOLCHAIN", toolchain);
    }
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
        apply_kani_env(&mut cmd);
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
    let export = dir.path().join("export.json");
    let mut cmd = Command::new("kani");
    cmd.arg("check.rs").args(export_flag_args()).arg("--export-json").arg(&export);
    cmd.current_dir(dir.path());
    apply_kani_env(&mut cmd);
    let out = process::run(cmd, Duration::from_secs(SHORT_TIMEOUT_S), token, None)?;
    let ok = out.exit_code == Some(0) && out.text.contains("VERIFICATION:- SUCCESSFUL");
    let mut result = json!({ "works": ok });
    if ok && let Some(problem) = export_problem(&export, dir.path()) {
        result["export_readable"] = json!(false);
        result["export_error"] = json!(problem);
    } else if ok {
        result["export_readable"] = json!(true);
    }
    if !ok {
        result["output_tail"] = json!(tail(&out.text, 20));
        result["hint"] = json!(
            "Kani could not verify a trivial harness. If the output mentions goto-cc or cbmc, \
             run `cargo kani setup` (or, for a development build, put CBMC on PATH)."
        );
    }
    Ok(result)
}

/// Why this server cannot read the export that a successful trivial run wrote (`None` if it can).
fn export_problem(export: &Path, dir: &Path) -> Option<String> {
    let text = match std::fs::read_to_string(export) {
        Ok(text) => text,
        Err(_) => return Some("Kani wrote no export for a successful run".into()),
    };
    let doc = match serde_json::from_str(&text)
        .map_err(anyhow::Error::from)
        .and_then(|raw| export::normalize(raw, dir))
    {
        Ok(doc) => doc,
        Err(e) => return Some(e.to_string()),
    };
    let harnesses = doc["harnesses"].as_array().map_or(0, Vec::len);
    (harnesses != 1 || doc["harnesses"][0]["outcome"]["verdict"] != "SUCCESS").then(|| {
        format!(
            "the plugin read {harnesses} harness(es) from the export of a run that verified one: the export \
             has a shape it does not understand"
        )
    })
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
        let mut cmd = Command::new(program);
        cmd.args(*args);
        apply_kani_env(&mut cmd);
        let out = cmd.output().ok()?;
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
    if let Some(toolchain) = std::env::var("RUSTUP_TOOLCHAIN").ok().or_else(kani_toolchain) {
        // Run `cargo kani` with this environment, so that rustup does not install the toolchain
        // a project pins in its rust-toolchain.toml (Kani does not use it).
        result["cli_env"] = json!(format!("RUSTUP_TOOLCHAIN={toolchain} RUSTUP_AUTO_INSTALL=0"));
    }
    if compatible {
        let toolchain = toolchain_check(token)?;
        if toolchain["works"] == false {
            result["compatible"] = json!(false);
            result["message"] = json!(format!(
                "Kani {} is supported, but it cannot verify a trivial harness.",
                installed.as_deref().unwrap_or("?")
            ));
        } else if toolchain["export_readable"] == false {
            result["compatible"] = json!(false);
            result["message"] = json!(format!(
                "Kani {} is supported, but this plugin cannot read its --export-json output ({}). Update the \
                 plugin, or use a Kani build whose export it supports.",
                installed.as_deref().unwrap_or("?"),
                toolchain["export_error"].as_str().unwrap_or("unknown shape")
            ));
        }
        result["export_flags"] = json!(export_flag_args().join(" "));
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
    // `kani list` does not take build options such as `--target-dir`: drop them, so the same
    // `extra_args` can be used for listing and verification.
    let (kept, ignored) = drop_list_unsupported(string_list(args, "extra_args")?);
    list_args.extend(auto_unstable(&target.dir, &kept));
    list_args.extend(kept);
    let out = process::run(
        target.command(Some("list"), &list_args),
        timeout_arg(args, SHORT_TIMEOUT_S),
        token,
        None,
    )?;
    let contents = std::fs::read_to_string(&list_file);
    if !existed {
        let _ = std::fs::remove_file(&list_file);
    }
    let full: Value = match contents {
        Ok(text) => serde_json::from_str(&text)?,
        Err(_) => {
            let errors = compiler_errors(&out.text);
            if errors.is_empty() {
                bail!(
                    "`kani list` did not produce results (exit code {:?}{}):\n{}",
                    out.exit_code,
                    if out.timed_out { ", timed out" } else { "" },
                    tail(&out.text, OUTPUT_TAIL_LINES)
                )
            }
            bail!(
                "`kani list` did not produce results: the crate does not compile. Compiler errors:\n{}",
                errors.join("\n\n")
            )
        }
    };
    let detail = args.get("detail").and_then(Value::as_str).unwrap_or("summary");
    let filter = args.get("filter").and_then(Value::as_str);
    if detail == "full" && filter.is_none() {
        return Ok(full);
    }
    let targets = contract_targets(&target.dir);
    let mut out = summarize_list(&full, filter, &targets);
    if !ignored.is_empty() {
        out["ignored_extra_args"] = json!(ignored);
    }
    Ok(out)
}

fn drop_list_unsupported(args: Vec<String>) -> (Vec<String>, Vec<String>) {
    // Build options `kani list` rejects ("unexpected argument").
    const WITH_VALUE: &[&str] =
        &["--target-dir", "--features", "-F", "--manifest-path", "--package", "-p"];
    const FLAGS: &[&str] = &["--no-default-features", "--all-features", "--workspace"];
    let (mut kept, mut ignored) = (vec![], vec![]);
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if WITH_VALUE.contains(&arg.as_str()) {
            ignored.push(arg);
            ignored.extend(iter.next());
        } else if FLAGS.contains(&arg.as_str())
            || WITH_VALUE.iter().any(|o| arg.starts_with(&format!("{o}=")))
        {
            ignored.push(arg);
        } else {
            kept.push(arg);
        }
    }
    (kept, ignored)
}

/// The last path segment of every `proof_for_contract(...)` target in the Rust sources under
/// `dir`. `kani list` can miss the harnesses of contracts on methods, so the summary also
/// checks the source.
fn contract_targets(dir: &Path) -> Vec<String> {
    let mut out = vec![];
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !name.starts_with("target") && !name.starts_with('.') {
                    todo.push(path);
                }
            } else if name.ends_with(".rs")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                for part in text.split("proof_for_contract(").skip(1) {
                    if let Some(end) = part.find(')') {
                        let target = part[..end].trim();
                        let last = target.rsplit("::").next().unwrap_or(target).trim();
                        out.push(last.to_string());
                    }
                }
            }
        }
    }
    out
}

/// A compact view of `kani list` output: totals, harness counts per file, contracts without
/// a harness, and, with `filter`, the matching harness and contract names.
fn summarize_list(full: &Value, filter: Option<&str>, source_targets: &[String]) -> Value {
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
    let has_source_harness = |function: &str| {
        let last = function.rsplit("::").next().unwrap_or(function);
        source_targets.iter().any(|t| t == last)
    };
    let without_harness: Vec<&Value> = contracts
        .iter()
        .filter(|c| c["harnesses"].as_array().is_none_or(Vec::is_empty))
        .filter(|c| !c["function"].as_str().is_some_and(has_source_harness))
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
            "No `proof_for_contract` harness was found for these contracts, by `kani list` or in \
             the source (matched by function name)."
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
    let extra = string_list(args, "extra_args")?;
    if let Ok(target) = Target::from_args(args) {
        out.extend(auto_unstable(&target.dir, &extra));
    }
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
    subcommand: Option<&str>,
    args: &[String],
    tool_args: &Value,
    token: Option<&Value>,
) -> Result<(Option<Value>, Output, Value)> {
    // Without `export_path` the export is kept under the plugin output directory (never /tmp), and
    // reported, so that it can be recorded as evidence.
    let export = match tool_args.get("export_path").and_then(Value::as_str) {
        Some(p) => PathBuf::from(p),
        None => plugin_out_dir(&target.dir).join(format!("export-{}.json", unix_time())),
    };
    if let Some(parent) = export.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create `{}`", parent.display()))?;
    }
    if export.exists() {
        std::fs::remove_file(&export)
            .with_context(|| format!("cannot replace `{}`", export.display()))?;
    }
    let mut all_args = export_flag_args();
    all_args.extend(["--export-json".into(), export.display().to_string()]);
    all_args.extend_from_slice(args);
    let cmd = target.command(subcommand, &all_args);
    let mut run_info =
        json!({ "command": command_line(&cmd), "cwd": target.dir.display().to_string() });
    let memory_mb = tool_args.get("memory_limit_mb").and_then(Value::as_u64);
    let out = process::run(cmd, timeout_arg(tool_args, DEFAULT_TIMEOUT_S), token, memory_mb)?;
    if let Some(log) = tool_args.get("log_path").and_then(Value::as_str) {
        write_creating_dirs(Path::new(log), &out.text)?;
        run_info["log_path"] = json!(log);
    }
    run_info["resources"] = out.record();
    if out.stopped() && !export.exists() {
        // Kani writes no export when it is stopped. Write one that says so, with what finished.
        let partial = partial::parse(&out.text);
        let started = unix_time().saturating_sub(out.elapsed_s);
        let harness = tool_args.get("harness").and_then(Value::as_str);
        let stopped = partial::stopped_export(&out, &partial, harness, started);
        write_creating_dirs(&export, &serde_json::to_string_pretty(&stopped)?)?;
        run_info["export_path"] = json!(export.display().to_string());
        run_info["stopped_export"] = json!(true);
        return Ok((None, out, run_info));
    }
    let doc = match std::fs::read_to_string(&export) {
        Ok(text) => {
            let mut raw: Value =
                serde_json::from_str(&text).context("malformed Kani JSON export")?;
            // Keep what the run used next to its results.
            raw["plugin_run"] = out.record();
            let _ = std::fs::write(&export, serde_json::to_string_pretty(&raw)?);
            Some(export::normalize(raw, &target.dir)?)
        }
        Err(_) => None,
    };
    if doc.is_some() {
        run_info["export_path"] = json!(export.display().to_string());
    }
    Ok((doc, out, run_info))
}

fn no_results_error(out: &Output) -> anyhow::Error {
    if out.memory_exceeded {
        return anyhow::anyhow!(
            "{}. Kani was stopped because it used more than the memory limit (`memory_limit_mb`, \
             `KANI_PLUGIN_MEMORY_MB`); it writes no results when killed. Shrink symbolic inputs first (a \
             `char`, `String`, or slice multiplies the state), lower unwind bounds, or stub callees. Last \
             lines of output:\n{}",
            out.killed_message().unwrap_or_default(),
            tail(&out.text, OUTPUT_TAIL_LINES)
        );
    }
    if out.timed_out {
        return anyhow::anyhow!(
            "{}. Kani was stopped (`timeout_s`); it writes no results when it is killed. \
             Verify fewer harnesses per call (or use `per_harness`), set `harness_timeout_s`, or reduce \
             input and unwind bounds. Last lines of output:\n{}",
            out.killed_message().unwrap_or_default(),
            tail(&out.text, OUTPUT_TAIL_LINES)
        );
    }
    let mut message =
        format!("Kani did not produce verification results (exit code {:?}).", out.exit_code);
    if out.text.contains("Failed to match the following harness") {
        message.push_str(
            " A `harness` filter matched nothing. With `exact` the fully qualified name is required \
             (`proofs::check_x`, as `kani_list_harnesses` shows it); without `exact` any substring works.",
        );
    }
    let errors = compiler_errors(&out.text);
    if errors.is_empty() {
        message.push_str(&format!(
            " This is usually a compilation error or an invalid argument; last lines of output:\n{}",
            tail(&out.text, OUTPUT_TAIL_LINES)
        ));
    } else {
        message.push_str(&format!(" Compiler errors:\n{}", errors.join("\n\n")));
    }
    anyhow::anyhow!(message)
}

/// Kani wrote no export. If the run was stopped by the timeout, report the harnesses that
/// finished, read from Kani's console output; otherwise the run failed.
fn partial_or_error(out: &Output, run_info: Value) -> Result<Value> {
    let partial = partial::parse(&out.text);
    if out.stopped() && !partial.finished.is_empty() {
        return Ok(json!({
            "run_state": if out.memory_exceeded { "KILLED" } else { "TIMEOUT" },
            "complete": false,
            "killed": out.killed_message(),
            "resources": out.record(),
            "harnesses": partial.finished,
            "unfinished": partial.unfinished,
            "note": format!(
                "Stopped after {}s (`timeout_s` or `memory_limit_mb`). These results were read from Kani's console \
                 output, because Kani writes no `--export-json` when it is killed: they have no \
                 proof-quality counts and cannot be recorded with evidence.py. Re-run the \
                 unfinished harnesses separately.",
                out.elapsed_s
            ),
            "run": run_info,
        }));
    }
    Err(no_results_error(out))
}

/// Run each harness in its own Kani process, so that a timeout or a memory kill loses only that
/// harness. `timeout_s` is the budget of the whole call; each harness gets `per_harness_timeout_s`.
fn verify_per_harness(args: &Value, token: Option<&Value>) -> Result<Value> {
    let started = std::time::Instant::now();
    let total = timeout_arg(args, DEFAULT_TIMEOUT_S);
    let per = args.get("per_harness_timeout_s").and_then(Value::as_u64).unwrap_or(900);
    let target = Target::from_args(args)?;
    let mut list_args = args.clone();
    list_args["detail"] = json!("full");
    list_args.as_object_mut().map(|o| o.remove("filter"));
    let listing = list_harnesses(&list_args, token)?;
    let filter = args.get("harness").and_then(Value::as_str);
    let mut names: Vec<String> = ["standard-harnesses", "contract-harnesses"]
        .iter()
        .flat_map(|k| listing[k].as_object().into_iter().flatten())
        .flat_map(|(_, v)| v.as_array().cloned().unwrap_or_default())
        .filter_map(|v| v.as_str().map(String::from))
        .filter(|n| filter.is_none_or(|f| n.contains(f)))
        .collect();
    names.sort();
    names.dedup();
    if names.is_empty() {
        bail!("no harness matches `{}`", filter.unwrap_or(""));
    }
    let dir = plugin_out_dir(&target.dir).join(format!("per-harness-{}", unix_time()));
    std::fs::create_dir_all(&dir)?;
    let (mut entries, mut not_run, mut first_doc) = (vec![], vec![], None::<Value>);
    let mut commands = vec![];
    for name in &names {
        let remaining = total.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            not_run.push(name.clone());
            continue;
        }
        let safe: String =
            name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let mut one = args.clone();
        one["harness"] = json!(name);
        one["export_path"] = json!(dir.join(format!("{safe}.json")).display().to_string());
        one["timeout_s"] = json!(remaining.as_secs().min(per));
        one["log_path"] = json!(dir.join(format!("{safe}.log")).display().to_string());
        let mut kani_args = vec!["--harness".to_string(), name.clone(), "--exact".to_string()];
        if let Some(n) = args.get("harness_timeout_s").and_then(Value::as_u64) {
            kani_args.extend([
                "-Z".into(),
                "unstable-options".into(),
                "--harness-timeout".into(),
                format!("{n}s"),
            ]);
        }
        let extra = string_list(args, "extra_args")?;
        kani_args.extend(auto_unstable(&target.dir, &extra));
        kani_args.extend(extra);
        let (doc, out, info) = run_with_export(&target, None, &kani_args, &one, token)?;
        commands.push(info["command"].clone());
        match doc {
            Some(d) => {
                for mut h in d["harnesses"].as_array().cloned().unwrap_or_default() {
                    h["plugin_run"] = out.record();
                    entries.push(h);
                }
                first_doc.get_or_insert(d);
            }
            None => {
                let kind = if out.memory_exceeded {
                    "OUT_OF_MEMORY"
                } else if out.timed_out {
                    "TIMEOUT"
                } else {
                    "CRASHED"
                };
                entries.push(json!({
                    "name": name, "file": "", "line": 0, "outcome": { "kind": kind },
                    "resources": { "verification_time_s": out.elapsed_s },
                    "failed_properties": [], "checks": {}, "covers": {}, "attributes": {},
                    "plugin_run": out.record(),
                }));
            }
        }
    }
    let mut merged = first_doc.unwrap_or_else(|| json!({}));
    merged["harnesses"] = json!(entries);
    merged["summary"] = json!({});
    // The first harness's record is not the record of the whole call.
    merged.as_object_mut().map(|o| o.remove("plugin_run"));
    let mut result = summary::summarize(&merged, false, summary::DEFAULT_MAX_FAILED);
    result["run_state"] = json!(if not_run.is_empty() { "COMPLETE" } else { "PARTIAL" });
    result["not_run"] = json!(not_run);
    result["per_harness"] = json!({ "dir": dir.display().to_string(), "harnesses": names.len() });
    result["note"] = json!(
        "Each harness ran in its own process; exports and logs are in `per_harness.dir`. Record a harness's \
         export with evidence.py, or merge them with `scripts/run-harnesses.py`."
    );
    Ok(result)
}

fn verify_harness(args: &Value, token: Option<&Value>) -> Result<Value> {
    if args.get("per_harness").and_then(Value::as_bool).unwrap_or(false) {
        return verify_per_harness(args, token);
    }
    let target = Target::from_args(args)?;
    let mut kani_args = run_options(args)?;
    if let Some(jobs) = args.get("jobs").and_then(Value::as_u64) {
        kani_args.extend(["--jobs".into(), jobs.to_string(), "--output-format=terse".into()]);
    }
    let (doc, out, run_info) = run_with_export(&target, None, &kani_args, args, token)?;
    let Some(doc) = doc else { return partial_or_error(&out, run_info) };
    let mut result = summary::summarize(&doc, false, summary::DEFAULT_MAX_FAILED);
    result["exit_code"] = json!(out.exit_code);
    result["run"] = run_info;
    Ok(result)
}

/// Counts of harnesses by verdict, among those with `automatic == generated`.
fn verdict_counts(doc: &Value, generated: bool) -> Value {
    let (mut total, mut success, mut failure) = (0, 0, 0);
    for h in doc["harnesses"].as_array().into_iter().flatten() {
        if (h["is_automatically_generated"] == true) != generated {
            continue;
        }
        total += 1;
        match h["outcome"]["verdict"].as_str() {
            Some("SUCCESS") => success += 1,
            Some("FAILURE") => failure += 1,
            _ => {}
        }
    }
    json!({ "total": total, "success": success, "failure": failure, "other": total - success - failure })
}

fn regex_args(args: &Value, key: &str, flag: &str) -> Result<Vec<String>> {
    let mut out = vec![];
    for pattern in string_list(args, key)? {
        if pattern.contains(' ') {
            bail!(
                "`{key}`: pattern `{pattern}` contains a space; Kani rejects spaces: write `.` instead"
            );
        }
        out.extend([flag.to_string(), pattern]);
    }
    Ok(out)
}

fn autoharness_tool(args: &Value, token: Option<&Value>) -> Result<Value> {
    let target = Target::from_args(args)?;
    let mut kani_args = vec!["-Z".to_string(), "autoharness".into()];
    kani_args.extend(regex_args(args, "include_patterns", "--include-pattern")?);
    kani_args.extend(regex_args(args, "exclude_patterns", "--exclude-pattern")?);
    if args.get("bounded_arguments").and_then(Value::as_bool).unwrap_or(false) {
        kani_args.push("--bounded-arguments".into());
    }
    for (key, flag) in [("slice_bound", "--slice-bound"), ("string_bound", "--string-bound")] {
        if let Some(n) = args.get(key).and_then(Value::as_u64) {
            kani_args.extend([flag.to_string(), n.to_string()]);
        }
    }
    if args.get("list_only").and_then(Value::as_bool).unwrap_or(false) {
        // `--list` writes `kani-list.json` into the working directory: leave no trace.
        let list_file = target.dir.join("kani-list.json");
        let existed = list_file.exists();
        kani_args.extend(["--list".to_string(), "--format".into(), "json".into()]);
        kani_args.extend(string_list(args, "extra_args")?);
        let out = process::run(
            target.command(Some("autoharness"), &kani_args),
            timeout_arg(args, SHORT_TIMEOUT_S),
            token,
            None,
        )?;
        if !existed {
            let _ = std::fs::remove_file(&list_file);
        }
        if !out.text.contains("Selected Function") && !out.text.contains("Skipped Function") {
            bail!(
                "`kani autoharness --list` printed no function tables (exit code {:?}{}):\n{}",
                out.exit_code,
                if out.timed_out { ", timed out" } else { "" },
                tail(&out.text, OUTPUT_TAIL_LINES)
            );
        }
        let filter = args.get("filter").and_then(Value::as_str);
        return Ok(autoharness::list_summary(&out.text, filter));
    }
    kani_args.extend(run_options(args)?);
    let (doc, out, run_info) =
        run_with_export(&target, Some("autoharness"), &kani_args, args, token)?;
    let Some(doc) = doc else { return partial_or_error(&out, run_info) };
    let failures_only = args.get("failures_only").and_then(Value::as_bool).unwrap_or(true);
    let mut result = summary::summarize(&doc, failures_only, summary::DEFAULT_MAX_FAILED);
    let (automatic, manual) = (verdict_counts(&doc, true), verdict_counts(&doc, false));
    if manual["total"].as_u64().unwrap_or(0) > 0 {
        result["note"] = json!(
            "AutoHarness also ran the crate's manual harnesses, first and whatever the patterns \
             say, and with a default unwind bound of 20 (kani#5103): a manual harness without its \
             own `#[kani::unwind]` can fail here but pass under `cargo kani`. Compare it with a \
             plain run before trusting such a failure."
        );
    }
    result["automatic_harnesses"] = automatic;
    result["manual_harnesses"] = manual;
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
    let (doc, out, run_info) = run_with_export(&target, None, &kani_args, args, token)?;
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
    let verdicts: Vec<&str> = summary["harnesses"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|h| h["verdict"].as_str())
        .collect();
    let mut note = if verdicts.is_empty() {
        "All selected harnesses passed; there is no counterexample to reproduce.".to_string()
    } else if !verdicts.contains(&"FAILURE") {
        format!(
            "No harness finished with a failure (verdicts: {}); there is no counterexample. \
             A TIMEOUT or OUT_OF_MEMORY result is inconclusive.",
            verdicts.join(", ")
        )
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
    fn finds_toolchain_file_in_any_ancestor() {
        let root = std::env::temp_dir().join(format!("kani-mcp-toolchain-{}", std::process::id()));
        let bin = root.join("target").join("kani").join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("kani"), "").unwrap();
        std::fs::write(
            root.join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"nightly-2026-10-23\"\n",
        )
        .unwrap();
        assert_eq!(
            toolchain_channel_above(&bin.join("kani")).as_deref(),
            Some("nightly-2026-10-23")
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn picks_the_export_flag_the_installed_kani_accepts() {
        let rejected = "error: invalid value 'export-json' for '--unstable <UNSTABLE_FEATURE>'";
        assert_eq!(export_flag_for(false, rejected), "unstable-options");
        assert_eq!(export_flag_for(true, ""), "export-json");
        assert_eq!(export_flag_for(false, "error: something else"), "export-json");
    }

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
        let out = summarize_list(&full, Some("check_a"), &[]);
        assert_eq!(out["standard_harnesses_per_file"]["a.rs"], 2);
        assert_eq!(out["contracts_without_harness"], json!(["f"]));
        assert_eq!(out["matching_standard_harnesses"], json!(["check_a"]));
        let out = summarize_list(&full, None, &["f".to_string()]);
        assert_eq!(out["contracts_without_harness"], json!([]));
    }

    #[test]
    fn drops_build_options_from_list() {
        let args =
            ["-Z", "function-contracts", "--target-dir", "/t", "--target-dir=/u"].map(String::from);
        let (kept, ignored) = drop_list_unsupported(args.to_vec());
        assert_eq!(kept, ["-Z", "function-contracts"]);
        assert_eq!(ignored, ["--target-dir", "/t", "--target-dir=/u"]);
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
                "kani_autoharness",
                "kani_map_crate",
                "kani_plan_verification",
                "kani_update_verification_map",
                "kani_explain_function",
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
