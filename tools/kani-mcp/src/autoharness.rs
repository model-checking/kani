// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reading the tables `kani autoharness` prints: which functions got an automatic harness, and
//! which were skipped and why. They are only available in the console output (`--list` has no
//! JSON form for them), so the rows are parsed from the table text.

use serde_json::{Value, json};
use std::collections::BTreeMap;

/// How many skipped functions to name per reason.
const NAMES_PER_REASON: usize = 10;

enum Table {
    None,
    Selected,
    Skipped,
}

/// Rows of the `Selected Function` and `Skipped Function` tables in `text`.
pub fn parse_tables(text: &str) -> (Vec<String>, Vec<(String, String)>) {
    let mut table = Table::None;
    let (mut selected, mut skipped) = (vec![], vec![]);
    for line in text.lines() {
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.contains(&"Selected Function") {
            table = Table::Selected;
        } else if cells.contains(&"Skipped Function") {
            table = Table::Skipped;
        } else if cells.contains(&"Kind of Automatic Harness") {
            table = Table::None;
        } else {
            match table {
                Table::Selected if cells.len() >= 2 => selected.push(cells[1].to_string()),
                Table::Skipped if cells.len() >= 3 => {
                    skipped.push((cells[1].to_string(), cells[2].to_string()))
                }
                _ => {}
            }
        }
    }
    (selected, skipped)
}

/// A compact view of `kani autoharness --list`: how many functions are selected, and the skipped
/// ones grouped by reason, with a few names each.
pub fn list_summary(text: &str, filter: Option<&str>) -> Value {
    let (selected, skipped) = parse_tables(text);
    let mut by_reason: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (function, reason) in &skipped {
        by_reason.entry(reason).or_default().push(function);
    }
    let skipped_json: Vec<Value> = by_reason
        .iter()
        .map(|(reason, functions)| {
            let functions: Vec<&&str> = functions
                .iter()
                .filter(|f| filter.is_none_or(|p| f.contains(p)))
                .take(NAMES_PER_REASON)
                .collect();
            json!({ "reason": reason, "count": by_reason[reason].len(), "examples": functions })
        })
        .collect();
    let selected_view: Vec<&String> =
        selected.iter().filter(|f| filter.is_none_or(|p| f.contains(p))).collect();
    let mut out = json!({
        "selected_count": selected.len(),
        "skipped_count": skipped.len(),
        "skipped_by_reason": skipped_json,
    });
    if filter.is_some() || selected.len() <= 50 {
        out["selected"] = json!(selected_view);
    } else {
        out["note"] = json!(
            "Pass `filter` to list the selected functions that contain a string; only counts and \
             skipped reasons are shown for large crates."
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLES: &str = "Kani generated automatic harnesses for 2 function(s):\n\
+-------+-------------------+\n| Crate | Selected Function |\n+===================+\n\
| c     | midpoint          |\n+-------+-------------------+\n| c     | Meters::add       |\n\
+-------+-------------------+\n\
Kani did not generate automatic harnesses for 2 function(s).\n\
+-------+----------------------+--------------------------+\n\
| Crate | Skipped Function     | Reason for Skipping      |\n+==========+\n\
| c     | <M as Clone>::clone  | Did not match provided filters |\n\
|-------+----------------------+--------------------------|\n\
| c     | gen                  | Generic function with no candidate |\n";

    #[test]
    fn parses_tables() {
        let (selected, skipped) = parse_tables(TABLES);
        assert_eq!(selected, ["midpoint", "Meters::add"]);
        assert_eq!(skipped.len(), 2);
        assert_eq!(skipped[0].0, "<M as Clone>::clone");
    }

    #[test]
    fn summarizes_by_reason() {
        let out = list_summary(TABLES, None);
        assert_eq!(out["selected_count"], 2);
        assert_eq!(out["skipped_by_reason"].as_array().unwrap().len(), 2);
        assert_eq!(out["selected"], json!(["midpoint", "Meters::add"]));
    }
}
