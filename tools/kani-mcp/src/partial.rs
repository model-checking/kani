// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Results of the harnesses that finished before a run was stopped.
//!
//! Kani writes `--export-json` only at the end of a run, so when a run hits its timeout the
//! export is lost. The console output still has the verdict of every harness that finished;
//! this module reads it, for both the regular and the `--jobs` (`Thread N:` prefixed) formats.

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
        if let Some(name) = line.strip_prefix("Checking harness ") {
            let name = name.trim_end_matches("...").trim().to_string();
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
    fn reads_parallel_output() {
        let text = "Thread 0: Checking harness a...\nThread 1: Checking harness b...\n\
                    Thread 1: VERIFICATION:- SUCCESSFUL\nThread 1: Verification Time: 0.1s\n";
        let p = parse(text);
        assert_eq!(p.finished, vec![json!({ "name": "b", "verdict": "SUCCESS", "time_s": 0.1 })]);
        assert_eq!(p.unfinished, vec!["a".to_string()]);
    }
}
