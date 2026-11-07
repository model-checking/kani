// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Results of the harnesses that finished before a run was stopped.
//!
//! Kani writes `--export-json` only at the end of a run, so when a run hits its timeout the
//! export is lost. The console output still has the verdict of every harness that finished;
//! this module reads it, for both the regular and the `--jobs` (`Thread N:` prefixed) formats.

use crate::process::Output;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct Partial {
    pub finished: Vec<Value>,
    pub unfinished: Vec<String>,
}

pub fn parse(text: &str) -> Partial {
    let mut current: BTreeMap<String, String> = BTreeMap::new();
    let mut failed: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut started: Vec<String> = vec![];
    let mut finished: Vec<Value> = vec![];
    for raw in text.lines() {
        let (key, line) = match raw.strip_prefix("Thread ").and_then(|r| r.split_once(": ")) {
            Some((n, rest)) if n.chars().all(|c| c.is_ascii_digit()) => (n.to_string(), rest),
            _ => (String::new(), raw),
        };
        // Manual harnesses: `Checking harness NAME...`; automatic ones (autoharness):
        // `Autoharness: Checking function NAME against all possible inputs...`.
        let started_name = line
            .strip_prefix("Checking harness ")
            .map(|n| n.trim_end_matches("..."))
            .or_else(|| {
                line.strip_prefix("Autoharness: Checking function ").map(|n| {
                    let n = n
                        .trim_end_matches("against all possible inputs...")
                        .trim_end_matches("...")
                        .trim();
                    n.strip_suffix("'s contract").unwrap_or(n)
                })
            });
        if let Some(name) = started_name {
            let name = name.trim().to_string();
            current.insert(key.clone(), name.clone());
            failed.remove(&key);
            started.push(name);
        } else if let Some(desc) = line.strip_prefix("Failed Checks: ") {
            failed.entry(key).or_default().push(json!({ "description": desc.trim() }));
        } else if let Some(rest) = line.trim_start().strip_prefix("File: ") {
            // ` File: "src/lib.rs", line 14, in naive_midpoint`
            if let Some(last) = failed.get_mut(&key).and_then(|f| f.last_mut()) {
                let file = rest.split('"').nth(1).unwrap_or_default();
                let line_no = rest.split("line ").nth(1).and_then(|r| r.split(',').next());
                last["location"] = json!(match line_no {
                    Some(l) => format!("{file}:{}", l.trim()),
                    None => file.to_string(),
                });
            }
        } else if let Some(verdict) = line.strip_prefix("VERIFICATION:- ") {
            if let Some(name) = current.get(&key) {
                let verdict = match verdict.trim() {
                    "SUCCESSFUL" => "SUCCESS",
                    "FAILED" => "FAILURE",
                    other => other,
                };
                let mut entry = json!({ "name": name, "verdict": verdict });
                if let Some(f) = failed.remove(&key) {
                    entry["failed"] = json!(f);
                }
                finished.push(entry);
            }
        } else if let Some(time) = line.strip_prefix("Verification Time: ")
            && let Some(last) = finished
                .iter_mut()
                .rev()
                .find(|h| current.get(&key) == h["name"].as_str().map(String::from).as_ref())
            && let Ok(t) = time.trim().trim_end_matches('s').parse::<f64>()
        {
            last["time_s"] = json!((t * 100.0).round() / 100.0);
        }
    }
    let done: Vec<&str> = finished.iter().filter_map(|h| h["name"].as_str()).collect();
    let unfinished = started.into_iter().filter(|s| !done.contains(&s.as_str())).collect();
    Partial { finished, unfinished }
}

/// `2026-11-07T01:51:53Z` for seconds since the Unix epoch.
pub fn iso_utc(secs: u64) -> String {
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// The export for a run that was stopped (memory or time) before Kani wrote its own: `run_state`
/// is `KILLED` or `TIMEOUT`, harnesses that finished keep their verdict (read from the console),
/// the harness that was running is OUT_OF_MEMORY or TIMEOUT, and the rest were not run. Mirrors
/// `kani_run.stopped_export`.
pub fn stopped_export(
    out: &Output,
    partial: &Partial,
    harness: Option<&str>,
    started: u64,
) -> Value {
    let kind = if out.memory_exceeded { "OUT_OF_MEMORY" } else { "TIMEOUT" };
    let entry = |name: &str, outcome: Value, failed: Vec<Value>, seconds: f64| {
        json!({
            "name": name, "crate_name": "", "file": "", "line": 0, "outcome": outcome,
            "resources": { "verification_time_s": seconds },
            "failed_properties": failed, "checks": {}, "covers": {}, "attributes": {},
            "is_automatically_generated": false, "is_bounded": false,
            "generated_concrete_test": false, "from_console_log": true,
        })
    };
    let mut entries: Vec<Value> = partial
        .finished
        .iter()
        .map(|f| {
            let failed = f["failed"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|p| {
                    json!({ "class": null, "description": p["description"].clone(),
                            "file": null, "line": null })
                })
                .collect();
            entry(
                f["name"].as_str().unwrap_or_default(),
                json!({ "kind": "COMPLETED", "verdict": f["verdict"].clone() }),
                failed,
                f["time_s"].as_f64().unwrap_or(0.0),
            )
        })
        .collect();
    let stopped = harness
        .map(String::from)
        .or_else(|| partial.unfinished.last().cloned())
        .or_else(|| entries.is_empty().then(|| "(the whole run)".to_string()));
    if let Some(name) = &stopped
        && !entries.iter().any(|e| e["name"] == name.as_str())
    {
        let mut e = entry(name, json!({ "kind": kind }), vec![], out.elapsed_s as f64);
        e["plugin_run"] = out.record();
        entries.push(e);
    }
    for name in partial.unfinished.iter().filter(|n| Some(*n) != stopped.as_ref()) {
        entries.push(entry(name, json!({ "kind": "NOT_RUN" }), vec![], 0.0));
    }
    let verdicts = |v: &str| entries.iter().filter(|e| e["outcome"]["verdict"] == v).count();
    json!({
        "export_shape": "flat", "configuration_known": false, "configuration": {},
        "export_notes": [
            crate::export::NO_CONFIGURATION_NOTE,
            "Kani was stopped before it wrote an export: this document was written by the plugin from \
             its console output.",
        ],
        "tools": { "kani": null },
        "run_state": if out.memory_exceeded { "KILLED" } else { "TIMEOUT" },
        "started_at": iso_utc(started),
        "summary": { "total": entries.len(), "successful": verdicts("SUCCESS"), "failed": verdicts("FAILURE") },
        "harnesses": entries,
        "plugin_run": out.record(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sequential_output() {
        let text = "Checking harness proofs::a...\nFailed Checks: attempt to add with overflow\n \
                    File: \"src/lib.rs\", line 14, in f\nVERIFICATION:- FAILED\nVerification Time: 0.05s\n\
                    Checking harness proofs::b...\nsome output\n";
        let p = parse(text);
        assert_eq!(
            p.finished,
            vec![json!({
                "name": "proofs::a", "verdict": "FAILURE", "time_s": 0.05,
                "failed": [{ "description": "attempt to add with overflow", "location": "src/lib.rs:14" }]
            })]
        );
        assert_eq!(p.unfinished, vec!["proofs::b".to_string()]);
    }

    #[test]
    fn reads_autoharness_output() {
        let text = "Thread 4: Autoharness: Checking function Meters::add against all possible inputs...\n\
                    Thread 4: VERIFICATION:- SUCCESSFUL\nThread 4: Verification Time: 2.5s\n\
                    Thread 5: Autoharness: Checking function <A as B>::f against all possible inputs...\n";
        let p = parse(text);
        assert_eq!(
            p.finished,
            vec![json!({ "name": "Meters::add", "verdict": "SUCCESS", "time_s": 2.5 })]
        );
        assert_eq!(p.unfinished, vec!["<A as B>::f".to_string()]);
    }

    #[test]
    fn reads_autoharness_contract_output() {
        let text = "Thread 1: Autoharness: Checking function count_down's contract against all possible inputs...\n\
                    Thread 1: VERIFICATION:- SUCCESSFUL\n";
        assert_eq!(parse(text).finished[0]["name"], "count_down");
    }

    #[test]
    fn formats_utc_times() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_794_016_313), "2026-11-07T01:51:53Z");
        assert_eq!(iso_utc(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn stopped_export_names_what_finished_and_what_was_running() {
        let out = Output {
            exit_code: None,
            text: String::new(),
            timed_out: false,
            memory_exceeded: true,
            elapsed_s: 41,
            peak_memory_mb: Some(9120),
            cpu_s: None,
            guard: "scope",
            limit_mb: Some(8192),
        };
        let partial =
            parse("Checking harness a...\nVERIFICATION:- SUCCESSFUL\nChecking harness b...\n");
        let doc = stopped_export(&out, &partial, None, 0);
        assert_eq!(doc["run_state"], "KILLED");
        assert_eq!(doc["harnesses"][0]["outcome"]["verdict"], "SUCCESS");
        assert_eq!(doc["harnesses"][1]["name"], "b");
        assert_eq!(doc["harnesses"][1]["outcome"]["kind"], "OUT_OF_MEMORY");
        assert_eq!(
            doc["plugin_run"]["killed_message"],
            "killed: memory (peak 9120 MB, limit 8192 MB) after 41s"
        );
        let doc = crate::export::normalize(doc, std::path::Path::new(".")).unwrap();
        let summary = crate::summary::summarize(&doc, false, 5);
        assert_eq!(summary["killed"][0]["reason"], "memory");
        assert_eq!(summary["run"]["peak_memory_mb"], 9120);
        assert_eq!(summary["all_ok"], false);
    }

    #[test]
    fn reads_parallel_output() {
        let text = "Thread 0: Checking harness a...\nThread 1: Checking harness b...\n\
                    Thread 1: VERIFICATION:- SUCCESSFUL\nThread 1: Verification Time: 0.1s\n";
        let p = parse(text);
        assert_eq!(p.finished, vec![json!({ "name": "b", "verdict": "SUCCESS", "time_s": 0.1 })]);
        assert_eq!(p.unfinished, vec!["a".to_string()]);
    }
}
