// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

//! These structs are a subset of the ones found in `rustc_errors::json`.
//! They are only used for deserialization of JSON output provided by libtest.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Diagnostic {
    _message: String,
    _code: Option<DiagnosticCode>,
    _level: String,
    _children: Vec<Diagnostic>,
    rendered: Option<String>,
}

#[derive(Deserialize)]
struct ArtifactNotification {
    #[allow(dead_code)]
    artifact: PathBuf,
}

#[derive(Deserialize)]
struct FutureIncompatReport {
    future_incompat_report: Vec<FutureBreakageItem>,
}

#[derive(Deserialize)]
struct FutureBreakageItem {
    diagnostic: Diagnostic,
}

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
struct DiagnosticSpanMacroExpansion {
    /// name of macro that was applied (e.g., "foo!" or "#[derive(Eq)]")
    _macro_decl_name: String,
}

#[derive(Deserialize, Clone)]
struct DiagnosticCode {
    /// The code itself.
    _code: String,
}

pub fn extract_rendered(output: &str) -> String {
    output
        .lines()
        .filter_map(|line| {
            if line.starts_with('{') {
                if let Ok(diagnostic) = serde_json::from_str::<Diagnostic>(line) {
                    diagnostic.rendered
                } else if let Ok(report) = serde_json::from_str::<FutureIncompatReport>(line) {
                    if report.future_incompat_report.is_empty() {
                        None
                    } else {
                        Some(format!(
                            "Future incompatibility report: {}",
                            report.future_incompat_report.into_iter().fold(
                                String::new(),
                                |mut output, item| {
                                    use std::fmt::Write;
                                    let _ = writeln!(output, "Future breakage diagnostic:");
                                    let s = item
                                        .diagnostic
                                        .rendered
                                        .unwrap_or_else(|| "Not rendered".to_string());
                                    let _ = write!(output, "{s}");
                                    output
                                }
                            )
                        ))
                    }
                } else if serde_json::from_str::<ArtifactNotification>(line).is_ok() {
                    // Ignore the notification.
                    None
                } else {
                    print!(
                        "failed to decode compiler output as json: line: {line}\noutput: {output}"
                    );
                    panic!()
                }
            } else {
                // preserve non-JSON lines, such as ICEs
                Some(format!("{line}\n"))
            }
        })
        .collect()
}
