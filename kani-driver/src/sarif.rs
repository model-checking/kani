// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::call_cbmc::ExitStatus;
use crate::cbmc_output_parser::{CheckStatus, Property, SourceLocation, TraceItem};
use crate::harness_runner::HarnessResult;
use crate::session::KaniSession;
use anyhow::{Context, Result};
use pathdiff::diff_paths;
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const SARIF_VERSION: &str = "2.1.0";
const SARIF_SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const TOOL_NAME: &str = "Kani";
const TOOL_INFO_URI: &str = "https://github.com/model-checking/kani";

impl KaniSession {
    pub fn write_sarif(&self, results: &[HarnessResult<'_>]) -> Result<()> {
        let Some(path) = &self.args.sarif else { return Ok(()) };
        let log = SarifLog::from_harness_results(results);
        write_sarif_file(path, &log)
    }
}

fn write_sarif_file(path: &Path, log: &SarifLog) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create SARIF output directory `{}`", parent.display())
        })?;
    }

    let file = File::create(path)
        .with_context(|| format!("Failed to create SARIF output file `{}`", path.display()))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, log)
        .with_context(|| format!("Failed to write SARIF output to `{}`", path.display()))?;
    writer.write_all(b"\n")?;
    Ok(())
}

#[derive(Serialize)]
struct SarifLog {
    version: &'static str,
    #[serde(rename = "$schema")]
    schema: &'static str,
    runs: Vec<Run>,
}

#[derive(Serialize)]
struct Run {
    tool: Tool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct Tool {
    driver: Driver,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Driver {
    name: &'static str,
    version: &'static str,
    information_uri: &'static str,
    rules: Vec<ReportingDescriptor>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportingDescriptor {
    id: String,
    short_description: Message,
}

#[derive(Serialize)]
struct Message {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    rule_id: String,
    level: &'static str,
    message: Message,
    locations: Vec<Location>,
    properties: ResultProperties,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultProperties {
    harness: String,
    property_name: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    physical_location: PhysicalLocation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PhysicalLocation {
    artifact_location: ArtifactLocation,
    region: Region,
}

#[derive(Serialize)]
struct ArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Region {
    start_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_column: Option<u32>,
}

impl SarifLog {
    fn from_harness_results(results: &[HarnessResult<'_>]) -> Self {
        let mut rules = BTreeMap::<String, ReportingDescriptor>::new();
        let mut sarif_results = Vec::new();

        for harness_result in results {
            let harness = harness_result.harness;
            let result = &harness_result.result;

            match &result.results {
                Ok(properties) => {
                    for prop in properties {
                        if prop.is_cover_property() || prop.is_code_coverage_property() {
                            continue;
                        }

                        let Some(level) = sarif_level(&prop.status) else { continue };
                        let rule_id = format!("kani.cbmc.{}", prop.property_id.class);

                        rules.entry(rule_id.clone()).or_insert_with(|| ReportingDescriptor {
                            id: rule_id.clone(),
                            short_description: Message {
                                text: format!("CBMC property `{}`", prop.property_id.class),
                            },
                        });

                        let (file, line, column) = best_location(prop).unwrap_or_else(|| {
                            (
                                relativize_path(&harness.original_file),
                                harness.original_start_line as u32,
                                None,
                            )
                        });

                        sarif_results.push(SarifResult {
                            rule_id,
                            level,
                            message: Message {
                                text: format!("[{}] {}", harness.pretty_name, prop.description),
                            },
                            locations: vec![Location {
                                physical_location: PhysicalLocation {
                                    artifact_location: ArtifactLocation { uri: file },
                                    region: Region { start_line: line, start_column: column },
                                },
                            }],
                            properties: ResultProperties {
                                harness: harness.pretty_name.clone(),
                                property_name: Some(prop.property_name()),
                            },
                        });
                    }
                }
                Err(exit_status) => {
                    let (rule_id, desc) = exit_status_rule(exit_status);
                    rules.entry(rule_id.to_string()).or_insert_with(|| ReportingDescriptor {
                        id: rule_id.to_string(),
                        short_description: Message { text: desc.to_string() },
                    });

                    sarif_results.push(SarifResult {
                        rule_id: rule_id.to_string(),
                        level: "error",
                        message: Message { text: format!("[{}] {desc}", harness.pretty_name) },
                        locations: vec![Location {
                            physical_location: PhysicalLocation {
                                artifact_location: ArtifactLocation {
                                    uri: relativize_path(&harness.original_file),
                                },
                                region: Region {
                                    start_line: harness.original_start_line as u32,
                                    start_column: None,
                                },
                            },
                        }],
                        properties: ResultProperties {
                            harness: harness.pretty_name.clone(),
                            property_name: None,
                        },
                    });
                }
            }
        }

        SarifLog {
            version: SARIF_VERSION,
            schema: SARIF_SCHEMA,
            runs: vec![Run {
                tool: Tool {
                    driver: Driver {
                        name: TOOL_NAME,
                        version: env!("CARGO_PKG_VERSION"),
                        information_uri: TOOL_INFO_URI,
                        rules: rules.into_values().collect(),
                    },
                },
                results: sarif_results,
            }],
        }
    }
}

fn sarif_level(status: &CheckStatus) -> Option<&'static str> {
    match status {
        CheckStatus::Failure => Some("error"),
        CheckStatus::Undetermined | CheckStatus::Unknown => Some("warning"),
        _ => None,
    }
}

fn exit_status_rule(exit_status: &ExitStatus) -> (&'static str, &'static str) {
    match exit_status {
        ExitStatus::Timeout => ("kani.cbmc.timeout", "CBMC timed out"),
        ExitStatus::OutOfMemory => ("kani.cbmc.oom", "CBMC ran out of memory"),
        ExitStatus::Other(_) => ("kani.cbmc.failed", "CBMC failed"),
    }
}

fn best_location(prop: &Property) -> Option<(String, u32, Option<u32>)> {
    if let Some(loc) = location_from_source_location(&prop.source_location) {
        return Some(loc);
    }

    prop.trace.as_ref().and_then(|trace| trace.iter().rev().find_map(trace_item_location))
}

fn trace_item_location(item: &TraceItem) -> Option<(String, u32, Option<u32>)> {
    let loc = item.source_location.as_ref()?;
    location_from_source_location(loc)
}

fn location_from_source_location(loc: &SourceLocation) -> Option<(String, u32, Option<u32>)> {
    let file = loc.file.as_deref()?;
    let line: u32 = loc.line.as_deref()?.parse().ok()?;
    let column = loc.column.as_deref().and_then(|c| c.parse().ok());
    Some((relativize_path(file), line, column))
}

fn relativize_path(file: &str) -> String {
    let file_path = PathBuf::from(file);
    let Ok(cur_dir) = env::current_dir() else { return file.to_string() };

    diff_paths(file_path, cur_dir)
        .unwrap_or_else(|| PathBuf::from(file))
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_cbmc::{FailedProperties, VerificationResult, VerificationStatus};
    use kani_metadata::{HarnessAttributes, HarnessKind, HarnessMetadata};
    use std::time::Duration;

    fn harness(pretty: &str) -> HarnessMetadata {
        HarnessMetadata {
            pretty_name: pretty.to_string(),
            mangled_name: "mangled".to_string(),
            crate_name: "krate".to_string(),
            original_file: "src/lib.rs".to_string(),
            original_start_line: 10,
            original_end_line: 20,
            goto_file: None,
            attributes: HarnessAttributes::new(HarnessKind::Proof),
            contract: None,
            has_loop_contracts: false,
            is_automatically_generated: false,
        }
    }

    fn failure_property() -> Property {
        Property {
            description: "assertion failed: x == 0".to_string(),
            property_id: crate::cbmc_output_parser::PropertyId {
                fn_name: Some("harness".to_string()),
                class: "assertion".to_string(),
                id: 1,
            },
            source_location: SourceLocation {
                file: Some("src/lib.rs".to_string()),
                line: Some("12".to_string()),
                column: Some("3".to_string()),
                function: Some("harness".to_string()),
            },
            status: CheckStatus::Failure,
            reach: None,
            trace: None,
        }
    }

    fn timeout_result() -> VerificationResult {
        VerificationResult {
            status: VerificationStatus::Failure,
            failed_properties: FailedProperties::None,
            results: Err(ExitStatus::Timeout),
            runtime: Duration::from_secs(1),
            generated_concrete_test: false,
            coverage_results: None,
        }
    }

    #[test]
    fn sarif_includes_failed_properties() {
        let harness = harness("my_harness");
        let result = VerificationResult {
            status: VerificationStatus::Failure,
            failed_properties: FailedProperties::PanicsOnly,
            results: Ok(vec![failure_property()]),
            runtime: Duration::from_secs(1),
            generated_concrete_test: false,
            coverage_results: None,
        };
        let harness_result = HarnessResult { harness: &harness, result };

        let log = SarifLog::from_harness_results(&[harness_result]);
        let v = serde_json::to_value(log).unwrap();

        assert_eq!(v["version"], SARIF_VERSION);
        assert_eq!(v["runs"][0]["tool"]["driver"]["name"], TOOL_NAME);
        assert_eq!(v["runs"][0]["results"].as_array().unwrap().len(), 1);

        let r = &v["runs"][0]["results"][0];
        assert_eq!(r["ruleId"], "kani.cbmc.assertion");
        assert_eq!(r["level"], "error");
        assert_eq!(r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"], "src/lib.rs");
        assert_eq!(r["locations"][0]["physicalLocation"]["region"]["startLine"], 12);
    }

    #[test]
    fn sarif_includes_timeouts() {
        let harness = harness("my_harness");
        let harness_result = HarnessResult { harness: &harness, result: timeout_result() };

        let log = SarifLog::from_harness_results(&[harness_result]);
        let v = serde_json::to_value(log).unwrap();
        assert_eq!(v["runs"][0]["results"].as_array().unwrap().len(), 1);

        let r = &v["runs"][0]["results"][0];
        assert_eq!(r["ruleId"], "kani.cbmc.timeout");
        assert_eq!(r["level"], "error");
        assert_eq!(r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"], "src/lib.rs");
        assert_eq!(r["locations"][0]["physicalLocation"]["region"]["startLine"], 10);
    }
}
