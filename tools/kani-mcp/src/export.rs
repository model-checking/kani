// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reads Kani's `--export-json` document, whichever Kani wrote it: the one place in this server
//! that knows the shapes. It mirrors `agent-plugin/scripts/kani_export.py`, which documents them.
//!
//! - *flat*: Kani since the RFC 0015 export (#4941). Has `harnesses[]`, the run `configuration` and
//!   each harness's `stubs`. Returned as it is.
//! - *tree*: Kani 0.68.0 and builds before #4941 (`harness_metadata`, `verification_results`, ...).
//!   Converted to the flat shape. What it does not record is marked, never guessed: the run
//!   configuration (`configuration_known` is false, so weakened checks cannot be detected), and the
//!   stubs of each harness (read from the `#[kani::stub(..)]` / `#[kani::stub_verified(..)]`
//!   attributes above the harness function; `attributes.stubs_source` says where they came from).

use anyhow::{Result, bail};
use serde_json::{Map, Value, json};
use std::path::Path;

pub const NO_CONFIGURATION_NOTE: &str = "This export does not record the run's configuration, so \
weakened checks (--no-*-checks, --cbmc-args, --prove-safety-only) cannot be detected from it. \
Compare with the command that produced it, or use a Kani that writes the flat export.";

/// Sections of the tree shape that are consumed by the conversion.
const TREE_ONLY: &[&str] = &[
    "harness_metadata",
    "verification_results",
    "property_details",
    "error_details",
    "cbmc",
    "coverage",
];

/// The flat form of an export, with `export_shape` and `configuration_known` added. `workspace` is
/// where harness sources are looked for (to recover the stubs of tree exports).
pub fn normalize(doc: Value, workspace: &Path) -> Result<Value> {
    if doc.get("harnesses").is_some() {
        let mut doc = doc;
        let known = doc.get("configuration").is_some();
        let obj = doc.as_object_mut().expect("a document with `harnesses` is an object");
        obj.entry("export_shape").or_insert(json!("flat"));
        obj.entry("configuration_known").or_insert(json!(known));
        return Ok(doc);
    }
    if doc.get("harness_metadata").is_some() && doc.get("verification_results").is_some() {
        return Ok(from_tree(&doc, workspace));
    }
    let keys: Vec<&str> =
        doc.as_object().map(|o| o.keys().map(String::as_str).collect()).unwrap_or_default();
    bail!(
        "unknown Kani export shape (top-level keys: {keys:?}). This plugin reads the flat export \
         (`harnesses`, Kani since #4941) and the tree export (`harness_metadata`, Kani 0.68.0 and \
         older builds)"
    )
}

/// Whether the export records the run configuration, so that weakened checks can be reported.
pub fn weakened_checks_detectable(doc: &Value) -> bool {
    doc["configuration_known"].as_bool().unwrap_or_else(|| doc.get("configuration").is_some())
}

fn norm(text: &Value) -> String {
    text.as_str().unwrap_or_default().replace(['_', '-', ' '], "").to_lowercase()
}

fn outcome(status: &Value) -> Value {
    match norm(status).as_str() {
        "success" => json!({ "kind": "COMPLETED", "verdict": "SUCCESS" }),
        "failure" => json!({ "kind": "COMPLETED", "verdict": "FAILURE" }),
        "timeout" => json!({ "kind": "TIMEOUT" }),
        "outofmemory" | "oom" => json!({ "kind": "OUT_OF_MEMORY" }),
        "crashed" | "crash" => json!({ "kind": "CRASHED" }),
        "killed" => json!({ "kind": "KILLED" }),
        _ => json!({
            "kind": "COMPLETED",
            "verdict": status.as_str().unwrap_or("UNKNOWN").to_uppercase()
        }),
    }
}

/// `<function>.<class>.<n>`, the flat shape's property id.
fn property_id(check: &Value) -> String {
    let part = |v: &Value| match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    };
    format!("{}.{}.{}", part(&check["function"]), part(&check["category"]), part(&check["id"]))
}

fn line_of(check: &Value) -> Value {
    // The flat export spells line numbers as strings.
    match &check["location"]["line"] {
        Value::String(s) if s.parse::<u64>().is_ok() => json!(s),
        Value::Number(n) => json!(n.to_string()),
        _ => Value::Null,
    }
}

fn ids(checks: &[&Value], statuses: &[&str]) -> Vec<String> {
    checks
        .iter()
        .filter(|c| statuses.contains(&norm(&c["status"]).as_str()))
        .map(|c| property_id(c))
        .collect()
}

/// The text of the attributes above the harness function `name`, which starts at `line` (1-based)
/// in `file`: the `fn` line, or an attribute line above it. A candidate file counts only if the
/// `fn` is where the export says (a relative path may exist in the wrong directory).
fn attribute_block(roots: &[&Path], file: &str, line: usize, name: &str) -> Option<String> {
    let function = name.rsplit("::").next().unwrap_or(name);
    let path = Path::new(file);
    for root in roots {
        let candidate = if path.is_absolute() { path.to_path_buf() } else { root.join(file) };
        let Ok(text) = std::fs::read_to_string(candidate) else { continue };
        let lines: Vec<&str> = text.lines().collect();
        if line == 0 || line > lines.len() {
            continue;
        }
        let Some(fn_index) =
            (line - 1..lines.len().min(line + 31)).find(|&k| declares(lines[k], function))
        else {
            continue;
        };
        // Up, until the end of the previous item.
        let mut block: Vec<&str> = vec![];
        let mut i = fn_index;
        while i > 0 {
            i -= 1;
            let code = lines[i].split("//").next().unwrap_or("").trim_end();
            if code.ends_with([';', '}', '{']) && !code.trim_start().starts_with('#') {
                break;
            }
            block.push(lines[i]);
        }
        block.reverse();
        return Some(block.join("\n"));
    }
    None
}

/// Whether `line` declares `fn <function>`.
fn declares(line: &str, function: &str) -> bool {
    let words: Vec<&str> = line
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect();
    words.windows(2).any(|w| w[0] == "fn" && w[1] == function)
}

/// Arguments of every `#[kani::<attr>(...)]` in `block`, split at top-level commas.
fn attribute_args(block: &str, attr: &str) -> Vec<Vec<String>> {
    let marker = format!("kani::{attr}");
    let mut out = vec![];
    let mut from = 0;
    while let Some(found) = block[from..].find(&marker) {
        from += found + marker.len();
        // `kani::stub` must not match `kani::stub_verified`: the name is followed by `(`.
        let Some(body) = block[from..].trim_start().strip_prefix('(') else { continue };
        let mut depth = 1usize;
        let Some(end) = body.char_indices().find_map(|(i, ch)| {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            (depth == 0).then_some(i)
        }) else {
            break;
        };
        let inner = &body[..end];
        let (mut args, mut depth, mut start) = (vec![], 0i32, 0usize);
        for (i, ch) in inner.char_indices() {
            match ch {
                '(' | '<' | '[' => depth += 1,
                ')' | '>' | ']' => depth -= 1,
                ',' if depth == 0 => {
                    args.push(inner[start..i].trim().to_string());
                    start = i + 1;
                }
                _ => {}
            }
        }
        args.push(inner[start..].trim().to_string());
        out.push(args);
    }
    out
}

fn stubs_from_source(
    roots: &[&Path],
    file: &str,
    line: usize,
    name: &str,
) -> (Vec<Value>, Vec<Value>, bool) {
    let Some(block) = attribute_block(roots, file, line, name) else {
        return (vec![], vec![], false);
    };
    let stubs = attribute_args(&block, "stub")
        .into_iter()
        .filter(|a| a.len() == 2)
        .map(|a| json!({ "original": a[0], "replacement": a[1] }))
        .collect();
    let verified = attribute_args(&block, "stub_verified")
        .into_iter()
        .filter(|a| a.len() == 1)
        .map(|a| json!(a[0]))
        .collect();
    (stubs, verified, true)
}

fn kind(raw: &Value) -> Value {
    match raw.as_str() {
        Some(s) if s.starts_with("ProofForContract") => {
            let target = s.split("target_fn:").nth(1).and_then(|t| t.split('"').nth(1));
            json!({ "ProofForContract": { "target_fn": target } })
        }
        _ => raw.clone(),
    }
}

fn from_tree(doc: &Value, workspace: &Path) -> Value {
    let recorded = doc["project"]["workspace_root"].as_str().map(Path::new);
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut roots: Vec<&Path> = vec![workspace];
    roots.extend(recorded);
    roots.push(&cwd);
    let results = doc["verification_results"]["results"].as_array().cloned().unwrap_or_default();
    let solvers: Vec<(String, Value)> = doc["cbmc"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| {
            (
                c["harness_id"].as_str().unwrap_or_default().to_string(),
                c["configuration"]["solver"].clone(),
            )
        })
        .collect();
    let mut harnesses = vec![];
    for m in doc["harness_metadata"].as_array().into_iter().flatten() {
        let name = m["pretty_name"].as_str().unwrap_or_default();
        let result = results.iter().find(|r| r["harness_id"].as_str() == Some(name));
        let all: Vec<&Value> =
            result.and_then(|r| r["checks"].as_array()).into_iter().flatten().collect();
        let plain: Vec<&Value> = all.iter().copied().filter(|c| c["category"] != "cover").collect();
        let covers: Vec<&Value> =
            all.iter().copied().filter(|c| c["category"] == "cover").collect();
        let failed: Vec<Value> = plain
            .iter()
            .filter(|c| norm(&c["status"]) == "failure")
            .map(|c| {
                json!({
                    "id": property_id(c), "description": c["description"], "class": c["category"],
                    "file": c["location"]["file"], "line": line_of(c), "status": "FAILURE",
                })
            })
            .collect();
        let unsupported: Vec<Value> = all
            .iter()
            .filter(|c| {
                c["category"].as_str().is_some_and(|k| k.to_lowercase().contains("unsupported"))
                    || norm(&c["status"]) == "unsupported"
            })
            .map(|c| {
                json!({ "description": c["description"], "file": c["location"]["file"], "line": line_of(c) })
            })
            .collect();
        let file = m["source"]["file"].as_str().unwrap_or_default();
        let start_line = m["source"]["start_line"].as_u64().unwrap_or(0);
        let (stubs, verified, stubs_source) = if m["is_automatically_generated"] == true {
            (vec![], vec![], "autoharness-generated")
        } else {
            match stubs_from_source(&roots, file, start_line as usize, name) {
                (s, v, true) => (s, v, "source-scan"),
                (s, v, false) => (s, v, "unknown"),
            }
        };
        let contract = if m["contract"]["contracted_function_name"].is_null() {
            Value::Null
        } else {
            m["contract"].clone()
        };
        let solver =
            solvers.iter().find(|(n, _)| n == name).map_or(Value::Null, |(_, s)| s.clone());
        harnesses.push(json!({
            "name": name, "crate_name": m["crate_name"], "file": file, "line": start_line,
            "contract": contract,
            "is_automatically_generated": m["is_automatically_generated"] == true,
            "has_loop_contracts": m["has_loop_contracts"] == true,
            "is_bounded": m["is_bounded"] == true,
            "attributes": {
                "kind": kind(&m["attributes"]["kind"]),
                "should_panic": m["attributes"]["should_panic"] == true,
                "solver": null, "unwind_value": null,
                "stubs": stubs, "verified_stubs": verified, "stubs_source": stubs_source,
            },
            "outcome": result.map_or(json!({ "kind": "NOT_RUN" }), |r| outcome(&r["status"])),
            "resolved_solver": solver, "resolved_unwind": null,
            "generated_concrete_test": false,
            "resources": {
                "verification_time_s": result.and_then(|r| r["duration_ms"].as_f64()).unwrap_or(0.0) / 1000.0
            },
            "n_properties": plain.len(), "n_failed": failed.len(),
            "failed_properties": failed, "unsupported_constructs": unsupported,
            "warnings": [], "warnings_truncated": 0,
            "checks": {
                "total": plain.len(), "success": ids(&plain, &["success"]).len(),
                "failure": ids(&plain, &["failure"]), "unreachable": ids(&plain, &["unreachable"]),
                "undetermined": ids(&plain, &["undetermined"]),
                "error": [], "unknown": [], "other": [],
            },
            "covers": {
                "total": covers.len(), "satisfied": ids(&covers, &["satisfied"]),
                "unsatisfiable": ids(&covers, &["unsatisfiable"]),
                "unreachable": ids(&covers, &["unreachable"]),
                "undetermined": ids(&covers, &["undetermined"]),
                "error": [], "unknown": [], "other": [],
            },
        }));
    }
    let count =
        |verdict: &str| harnesses.iter().filter(|h| h["outcome"]["verdict"] == verdict).count();
    let completed = doc["verification_results"]["summary"]["status"] == "completed"
        && harnesses.iter().all(|h| h["outcome"]["kind"] != "NOT_RUN");
    let started = doc["metadata"]["timestamp"]
        .as_str()
        .map(|t| format!("{}Z", t.chars().take(19).collect::<String>()));
    let mut tools = doc["tools"].as_object().cloned().unwrap_or_default();
    if let Some(v) = doc["metadata"]["kani_version"].as_str() {
        tools.insert("kani".into(), json!(v));
    }
    let mut out: Map<String, Value> = doc
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(k, _)| !TREE_ONLY.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    out.insert("export_shape".into(), json!("tree"));
    out.insert("configuration_known".into(), json!(false));
    out.insert("configuration".into(), json!({}));
    out.insert("export_notes".into(), json!([NO_CONFIGURATION_NOTE]));
    out.insert(
        "summary".into(),
        json!({ "total": harnesses.len(), "successful": count("SUCCESS"), "failed": count("FAILURE") }),
    );
    out.insert("harnesses".into(), json!(harnesses));
    out.insert("run_state".into(), json!(if completed { "COMPLETE" } else { "PARTIAL" }));
    out.insert("started_at".into(), started.map_or(Value::Null, Value::from));
    out.insert("tools".into(), Value::Object(tools));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TREE: &str = include_str!("../../../agent-plugin/tests/fixtures/exports/probe-tree.json");
    const FLAT: &str = include_str!("../../../agent-plugin/tests/fixtures/exports/probe-flat.json");

    fn probe_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agent-plugin/tests/fixtures/export-probe")
    }

    fn load(text: &str) -> Value {
        normalize(serde_json::from_str(text).unwrap(), &probe_dir()).unwrap()
    }

    #[test]
    fn tree_and_flat_agree() {
        let (tree, flat) = (load(TREE), load(FLAT));
        assert_eq!(tree["export_shape"], "tree");
        assert_eq!(flat["export_shape"], "flat");
        assert!(!weakened_checks_detectable(&tree));
        assert!(weakened_checks_detectable(&flat));
        assert_eq!(tree["run_state"], flat["run_state"]);
        assert_eq!(tree["started_at"], flat["started_at"]);
        let (t, f) = (tree["harnesses"].as_array().unwrap(), flat["harnesses"].as_array().unwrap());
        assert_eq!(t.len(), 4);
        for (a, b) in t.iter().zip(f) {
            assert_eq!(a["name"], b["name"]);
            assert_eq!(a["outcome"], b["outcome"], "{}", a["name"]);
            assert_eq!(a["checks"]["unreachable"], b["checks"]["unreachable"], "{}", a["name"]);
            assert_eq!(a["covers"]["unsatisfiable"], b["covers"]["unsatisfiable"], "{}", a["name"]);
            assert_eq!(a["failed_properties"][0]["id"], b["failed_properties"][0]["id"]);
            assert_eq!(a["attributes"]["stubs"], b["attributes"]["stubs"], "{}", a["name"]);
            assert_eq!(a["attributes"]["kind"], b["attributes"]["kind"], "{}", a["name"]);
            assert_eq!(a["contract"].is_null(), b["contract"].is_null());
        }
    }

    #[test]
    fn summaries_agree_and_tree_says_what_it_cannot_detect() {
        let (tree, flat) = (
            crate::summary::summarize(&load(TREE), false, 5),
            crate::summary::summarize(&load(FLAT), false, 5),
        );
        for key in
            ["total", "successful", "failed", "inconclusive", "by_verdict", "all_ok", "run_state"]
        {
            assert_eq!(tree[key], flat[key], "{key}");
        }
        for (a, b) in
            tree["harnesses"].as_array().unwrap().iter().zip(flat["harnesses"].as_array().unwrap())
        {
            for key in
                ["name", "verdict", "failed", "unreachable_checks", "unsatisfiable_covers", "flags"]
            {
                assert_eq!(a[key], b[key], "{}: {key}", a["name"]);
            }
        }
        assert_eq!(
            tree["harnesses"][3]["trusted"]["stubs"],
            flat["harnesses"][3]["trusted"]["stubs"]
        );
        assert_eq!(tree["harnesses"][3]["trusted"]["stubs_source"], "source-scan");
        assert_eq!(flat["weakened_checks_detectable"], true);
        assert_eq!(tree["weakened_checks_detectable"], false);
        assert!(tree["weakened_checks_note"].as_str().unwrap().contains("cannot be detected"));
    }

    #[test]
    fn stubs_come_from_the_source_and_say_so() {
        let tree = load(TREE);
        let stubbed = &tree["harnesses"][3]["attributes"];
        assert_eq!(stubbed["stubs"][0]["original"], "expensive");
        assert_eq!(stubbed["stubs"][0]["replacement"], "stub_expensive");
        assert_eq!(stubbed["verified_stubs"][0], "checked_double");
        assert_eq!(stubbed["stubs_source"], "source-scan");
    }

    #[test]
    fn missing_source_is_unknown_not_guessed() {
        let doc: Value = serde_json::from_str(TREE).unwrap();
        let tree = normalize(doc, Path::new("/nonexistent")).unwrap();
        assert_eq!(tree["harnesses"][3]["attributes"]["stubs_source"], "unknown");
    }

    #[test]
    fn unknown_shapes_are_errors() {
        let err = normalize(json!({ "something": 1 }), Path::new(".")).unwrap_err();
        assert!(err.to_string().contains("unknown Kani export shape"));
    }
}
