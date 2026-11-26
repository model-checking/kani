// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! MCP tools for the Crate Verifiability Map. The map itself is built by the plugin's
//! `scripts/map-crate.py` (one implementation, shared with the CLI): these tools run it and return
//! compact answers, so an agent never has to load a whole map into its context. The map file stays
//! on disk; tools take its path.

use crate::process;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const SCRIPT_TIMEOUT_S: u64 = 1800;
/// How many functions a plan lists before it summarizes the rest.
const DEFAULT_PLAN_ITEMS: usize = 40;

/// The plugin's `scripts/` directory: `KANI_PLUGIN_DIR` (the plugin root), the `--plugin-dir`
/// argument, or the nearest ancestor of the working directory or of this executable that holds
/// `scripts/map-crate.py` (alone or under `agent-plugin/`).
pub fn scripts_dir() -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = vec![];
    if let Some(dir) = std::env::var_os("KANI_PLUGIN_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    let mut starts: Vec<PathBuf> = vec![];
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        starts.extend(exe.parent().map(Path::to_path_buf));
    }
    for start in starts {
        for dir in start.ancestors() {
            candidates.push(dir.to_path_buf());
            candidates.push(dir.join("agent-plugin"));
        }
    }
    candidates
        .into_iter()
        .map(|c| c.join("scripts"))
        .find(|s| s.join("map-crate.py").is_file())
        .context(
            "cannot find the plugin's scripts/map-crate.py. Set KANI_PLUGIN_DIR to the plugin directory \
             (the one that contains `scripts/` and `plugin.json`), or run the server from inside it. \
             The CLI works without the server: `scripts/map-crate.py map . --out map.json`.",
        )
}

fn run_script(args: &[String], timeout_s: u64, token: Option<&Value>) -> Result<String> {
    let script = scripts_dir()?.join("map-crate.py");
    let mut cmd = Command::new("python3");
    cmd.arg(&script).args(args);
    let out = process::run(cmd, Duration::from_secs(timeout_s), token, None)?;
    if out.stopped() {
        bail!("map-crate.py was stopped after {}s (`timeout_s`)", out.elapsed_s);
    }
    if out.exit_code != Some(0) {
        let message = out.text.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("no output");
        bail!("map-crate.py failed (exit code {:?}): {}", out.exit_code, message);
    }
    Ok(out.text)
}

fn string_list(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(String::from))
        .collect()
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key).and_then(Value::as_str).with_context(|| format!("missing `{key}` argument"))
}

fn timeout(args: &Value) -> u64 {
    args.get("timeout_s").and_then(Value::as_u64).unwrap_or(SCRIPT_TIMEOUT_S)
}

fn load_map(path: &str) -> Result<Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read map `{path}`"))?;
    serde_json::from_str(&text).with_context(|| format!("`{path}` is not valid JSON"))
}

/// A compact description of a map: counts, uncertainty, recursive components, and the
/// functions that are ready to verify next.
fn summarize_map(map: &Value, map_path: &str) -> Value {
    let functions = map["functions"].as_array().cloned().unwrap_or_default();
    let recursive: Vec<Value> = map["components"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["recursive"] == true)
        .take(8)
        .map(|c| json!({ "id": c["id"], "members": c["members"] }))
        .collect();
    let ready: Vec<&str> = functions
        .iter()
        .filter(|f| {
            f["verification"]["status"] != "verified"
                && f["unverified_callees"].as_array().is_none_or(Vec::is_empty)
        })
        .take(10)
        .filter_map(|f| f["id"].as_str())
        .collect();
    json!({
        "map_path": map_path,
        "schema_version": map["schema_version"],
        "call_graph": map["configuration"]["call_graph"],
        "summary": map["summary"],
        "recursive_components": recursive,
        "ready_to_verify_first": ready,
        "uncertainty": map["uncertainty"],
        "next": "kani_plan_verification for a staged plan; kani_explain_function for one function. A map is a \
                 planning aid, not a proof: `verified` means a fresh harness that provably reaches the \
                 function succeeded, under its recorded bounds, assumptions, and stubs.",
    })
}

pub fn map_crate(args: &Value, token: Option<&Value>) -> Result<Value> {
    let path = require_str(args, "path")?;
    let root = std::fs::canonicalize(path).with_context(|| format!("cannot access `{path}`"))?;
    let out = match args.get("map_path").and_then(Value::as_str) {
        Some(p) => PathBuf::from(p),
        None => {
            let dir = if root.is_dir() {
                root.clone()
            } else {
                root.parent().unwrap_or(&root).to_path_buf()
            };
            crate::plugin_out_dir(&dir).join("verifiability-map.json")
        }
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut cli = vec!["map".to_string(), root.display().to_string()];
    let results = string_list(args, "results");
    if !results.is_empty() {
        cli.push("--results".into());
        cli.extend(results);
    }
    cli.extend([
        "--callgraph".to_string(),
        args.get("callgraph").and_then(Value::as_str).unwrap_or("auto").to_string(),
        "--out".to_string(),
        out.display().to_string(),
    ]);
    run_script(&cli, timeout(args), token)?;
    // `update` needs the workspace root again; keep it next to the map.
    let workspace =
        if root.is_dir() { root.clone() } else { root.parent().unwrap_or(&root).to_path_buf() };
    std::fs::write(sidecar(&out), workspace.display().to_string())?;
    Ok(summarize_map(&load_map(&out.display().to_string())?, &out.display().to_string()))
}

fn sidecar(map: &Path) -> PathBuf {
    let mut name = map.as_os_str().to_os_string();
    name.push(".root");
    PathBuf::from(name)
}

pub fn plan(args: &Value, token: Option<&Value>) -> Result<Value> {
    let map_path = require_str(args, "map_path")?;
    let mut cli = vec!["plan".to_string(), map_path.to_string()];
    for goal in string_list(args, "goals") {
        cli.extend(["--goal".to_string(), goal]);
    }
    let text = run_script(&cli, timeout(args), token)?;
    let mut plan: Value =
        serde_json::from_str(&text).context("map-crate.py printed no JSON plan")?;
    let limit =
        args.get("max_items").and_then(Value::as_u64).map_or(DEFAULT_PLAN_ITEMS, |n| n as usize);
    let mut shown = 0usize;
    let mut omitted = 0usize;
    for stage in plan["stages"].as_array_mut().into_iter().flatten() {
        let items = stage["functions"].as_array_mut().unwrap();
        let keep = limit.saturating_sub(shown).min(items.len());
        omitted += items.len() - keep;
        items.truncate(keep);
        shown += keep;
    }
    plan["stages"] = Value::Array(
        plan["stages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["functions"].as_array().is_some_and(|f| !f.is_empty()))
            .cloned()
            .collect(),
    );
    if omitted > 0 {
        plan["omitted_functions"] = json!(omitted);
        plan["omitted_note"] = json!(
            "The plan lists the first functions of each stage; raise `max_items`, or narrow it with `goals`."
        );
    }
    Ok(plan)
}

pub fn update(args: &Value, token: Option<&Value>) -> Result<Value> {
    let map_path = require_str(args, "map_path")?;
    let results = string_list(args, "results");
    if results.is_empty() {
        bail!("`results` must list at least one --export-json file or evidence bundle");
    }
    let root = match args.get("root").and_then(Value::as_str) {
        Some(r) => r.to_string(),
        None => std::fs::read_to_string(sidecar(Path::new(map_path))).context(
            "the workspace root is unknown: pass `root`, or build the map with kani_map_crate (which records it)",
        )?,
    };
    let mut cli = vec![
        "update".to_string(),
        map_path.to_string(),
        "--root".to_string(),
        root,
        "--results".into(),
    ];
    cli.extend(results);
    run_script(&cli, timeout(args), token)?;
    Ok(summarize_map(&load_map(map_path)?, map_path))
}

pub fn explain(args: &Value, token: Option<&Value>) -> Result<Value> {
    let map_path = require_str(args, "map_path")?;
    let function = require_str(args, "function")?;
    let text = run_script(
        &["explain".to_string(), map_path.to_string(), function.to_string()],
        timeout(args),
        token,
    )?;
    serde_json::from_str(&text).context("map-crate.py printed no JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_a_map() {
        let map = json!({
            "schema_version": "0.1.0",
            "configuration": { "call_graph": "text-scan" },
            "summary": { "functions": 2 },
            "components": [{ "id": "scc0", "members": ["a"], "recursive": true },
                           { "id": "scc1", "members": ["b"], "recursive": false }],
            "functions": [
                { "id": "a", "verification": { "status": "verified" }, "unverified_callees": [] },
                { "id": "b", "verification": { "status": "not_attempted" }, "unverified_callees": [] },
                { "id": "c", "verification": { "status": "not_attempted" }, "unverified_callees": ["b"] }
            ],
            "uncertainty": [{ "flag": "approximate-call-graph", "reason": "r" }]
        });
        let s = summarize_map(&map, "m.json");
        assert_eq!(s["ready_to_verify_first"], json!(["b"]));
        assert_eq!(s["recursive_components"].as_array().unwrap().len(), 1);
        assert_eq!(s["call_graph"], "text-scan");
    }

    #[test]
    fn requires_arguments() {
        assert!(require_str(&json!({}), "map_path").is_err());
        assert!(update(&json!({ "map_path": "m.json", "results": [] }), None).is_err());
    }
}
