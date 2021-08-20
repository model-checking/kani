// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Logging for RMC to write structured logs to a log file.
use crate::btree_string_map;
use rustc_serialize::json::*;
use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;
use tracing::{debug, warn};

pub use crate::{rmc_debug, rmc_log, rmc_warn};

/// A line in the log file.
struct LogLine {
    log_type: LogType,
    message: String,
}

impl ToJson for LogLine {
    fn to_json(&self) -> Json {
        let output = btree_string_map![
            ("log_type", self.log_type.to_json()),
            ("message", self.message.to_json())
        ];

        Json::Object(output)
    }
}

/// The kind of log message.
#[derive(Debug)]
enum LogType {
    Debug,
    Log,
    Warning(WarningType),
}

impl ToJson for LogType {
    fn to_json(&self) -> Json {
        match self {
            LogType::Debug => {
                let output = btree_string_map![("type", "DEBUG".to_json())];
                Json::Object(output)
            }
            LogType::Log => {
                let output = btree_string_map![("type", "LOG".to_json())];
                Json::Object(output)
            }
            LogType::Warning(warning_type) => {
                let output = btree_string_map![
                    ("type", "WARNING".to_json()),
                    ("warning_kind", warning_type.to_json())
                ];
                Json::Object(output)
            }
        }
    }
}

/// The kind of warning message.
#[derive(Debug)]
pub enum WarningType {
    Concurrency,
    GlobalAsm,
    MissingSymbol,
    TypeMismatch,
    Unsupported,
}

impl ToJson for WarningType {
    fn to_json(&self) -> Json {
        format!("{:?}", self).to_json()
    }
}

/// Writes a given log line to the log file specified through
/// the the environment variable RMC_LOG_FILE.
/// If this is not set, continues without failing.
fn write_to_log_file(log_line: LogLine) {
    let line = format!("{}\n", log_line.to_json().to_string());
    match env::var("RMC_LOG_FILE") {
        Err(_) => (),
        Ok(path) => {
            OpenOptions::new().append(true).create(true).open(&path)
                .expect(&format!("Internal error: Unable to open log file at location {}; try specifying a different location for the log file using the `--save-logs` flag. If this issue persists, file a ticket at https://github.com/model-checking/rmc/issues/new?assignees=&labels=bug&template=bug_report.md", &path))
                .write_all(line.as_bytes())
                .expect(&format!("Internal error: Error writing to log file."))
        }
    }
}

#[macro_export]
macro_rules! rmc_debug {
    ($( $parts:expr ),*) => {
        let message = rustc_middle::ty::print::with_no_trimmed_paths(|| format!($( $parts, )*));
        crate::gotoc::logging::write_debug(message);
    }
}

pub fn write_debug(message: String) {
    debug!("RMC [DEBUG]: {}", message);
    write_to_log_file(LogLine { log_type: LogType::Debug, message: message })
}

#[macro_export]
macro_rules! rmc_log {
    ($( $parts:expr ),*) => {
        let message = rustc_middle::ty::print::with_no_trimmed_paths(|| format!($( $parts, )*));
        crate::gotoc::logging::write_log(message);
    }
}

pub fn write_log(message: String) {
    debug!("RMC [LOG]: {}", message);
    write_to_log_file(LogLine { log_type: LogType::Log, message: message })
}

#[macro_export]
macro_rules! rmc_warn {
    ($warning_type:expr, $( $rest:expr ),*) => {
        let rest_string = rustc_middle::ty::print::with_no_trimmed_paths(|| format!($( $rest, )*));
        crate::gotoc::logging::write_warning($warning_type, rest_string);
    }
}

pub fn write_warning(warning_type: WarningType, message: String) {
    warn!("RMC [WARNING] <{:?}>: {}", warning_type, message);
    write_to_log_file(LogLine { log_type: LogType::Warning(warning_type), message: message })
}
