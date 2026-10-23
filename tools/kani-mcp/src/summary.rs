// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Compact summary of a Kani `--export-json` document (RFC 0015).
//!
//! This mirrors `agent-plugin/scripts/summarize-results.py`, so agents get the same shape
//! whether or not the MCP server is available; the `agent-plugin` smoke test checks that the
//! two agree. Keep them in sync.

use serde_json::{Map, Value, json};

pub const DEFAULT_MAX_FAILED: usize = 5;

fn verdict_of(harness: &Value) -> Value {
    let outcome = &harness["outcome"];
    let field = if outcome["kind"] == "COMPLETED" { "verdict" } else { "kind" };
    outcome.get(field).cloned().unwrap_or(json!("UNKNOWN"))
}

fn location(prop: &Value) -> Option<String> {
    let file = prop["file"].as_str()?;
    Some(match prop["line"].as_str() {
        Some(line) => format!("{file}:{line}"),
        None => file.to_string(),
    })
}

fn len(value: &Value) -> usize {
    value.as_array().map_or(0, Vec::len)
}

/// Insert `value` unless it is "empty": null, zero, or false. Matches Python truthiness.
fn insert_nonempty(out: &mut Map<String, Value>, key: &str, value: Value) {
    let empty = match &value {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        _ => false,
    };
    if !empty {
        out.insert(key.to_string(), value);
    }
}

fn summarize_harness(harness: &Value, max_failed: usize) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), harness["name"].clone());
    out.insert("verdict".into(), verdict_of(harness));
    out.insert(
        "location".into(),
        json!(format!("{}:{}", harness["file"].as_str().unwrap_or_default(), harness["line"])),
    );
    let time = harness["resources"]["verification_time_s"].as_f64().unwrap_or(0.0);
    out.insert("time_s".into(), json!((time * 100.0).round() / 100.0));
    if let Some(message) = harness["outcome"]["message"].as_str().filter(|m| !m.is_empty()) {
        out.insert("message".into(), json!(message));
    }
    let failed = harness["failed_properties"].as_array().cloned().unwrap_or_default();
    if !failed.is_empty() {
        let shown: Vec<Value> = failed
            .iter()
            .take(max_failed)
            .map(|p| {
                let mut entry = Map::new();
                entry.insert("class".into(), p["class"].clone());
                entry.insert("description".into(), p["description"].clone());
                if let Some(loc) = location(p) {
                    entry.insert("location".into(), json!(loc));
                }
                Value::Object(entry)
            })
            .collect();
        out.insert("failed".into(), json!(shown));
        if failed.len() > max_failed {
            out.insert("failed_omitted".into(), json!(failed.len() - max_failed));
        }
    }
    let checks = &harness["checks"];
    let covers = &harness["covers"];
    let warnings =
        len(&harness["warnings"]) as u64 + harness["warnings_truncated"].as_u64().unwrap_or(0);
    for (key, value) in [
        ("checks_total", checks["total"].clone()),
        ("unreachable_checks", json!(len(&checks["unreachable"]))),
        ("undetermined_checks", json!(len(&checks["undetermined"]))),
        ("covers_total", covers["total"].clone()),
        ("unsatisfiable_covers", json!(len(&covers["unsatisfiable"]))),
        ("unreachable_covers", json!(len(&covers["unreachable"]))),
        ("unsupported_constructs", json!(len(&harness["unsupported_constructs"]))),
        ("warnings", json!(warnings)),
    ] {
        insert_nonempty(&mut out, key, value);
    }
    let flags: Vec<&str> = [
        ("automatically_generated", harness["is_automatically_generated"] == true),
        ("bounded", harness["is_bounded"] == true),
        ("contract", !harness["contract"].is_null()),
        ("should_panic", harness["attributes"]["should_panic"] == true),
        ("concrete_test_generated", harness["generated_concrete_test"] == true),
    ]
    .into_iter()
    .filter_map(|(name, on)| on.then_some(name))
    .collect();
    if !flags.is_empty() {
        out.insert("flags".into(), json!(flags));
    }
    Value::Object(out)
}

pub fn summarize(doc: &Value, failures_only: bool, max_failed: usize) -> Value {
    let harnesses: Vec<Value> = doc["harnesses"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|h| summarize_harness(h, max_failed))
        .filter(|h| !failures_only || h["verdict"] != "SUCCESS")
        .collect();
    let summary = &doc["summary"];
    let count = |key: &str| summary[key].as_u64().unwrap_or(0);
    let mut out = json!({
        "kani": doc["tools"]["kani"],
        "run_state": doc["run_state"],
        "total": count("total"),
        "successful": count("successful"),
        "failed": count("failed"),
        "harnesses": harnesses,
    });
    let unmatched = &doc["harness_selection"]["unmatched_filters"];
    if len(unmatched) > 0 {
        out["unmatched_filters"] = unmatched.clone();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../../../tests/json-handler/schema-validation/fixture-completed-failure.json"
    );

    #[test]
    fn summarizes_failure_fixture() {
        let doc: Value = serde_json::from_str(FIXTURE).unwrap();
        let summary = summarize(&doc, false, 1);
        let harness = &summary["harnesses"][0];
        assert_eq!(harness["verdict"], "FAILURE");
        assert_eq!(harness["failed"].as_array().unwrap().len(), 1);
        assert_eq!(harness["failed_omitted"], 1);
        assert_eq!(harness["failed"][0]["location"], "test.rs:10");
        assert_eq!(harness["unsatisfiable_covers"], 1);
        assert!(harness.get("unreachable_checks").is_none());
    }
}
