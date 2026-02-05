// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde_json::{Value, json};
use std::path::PathBuf;

/// A handler for building and exporting JSON data structures.
///
/// `JsonHandler` provides a convenient interface for constructing JSON objects,
/// adding key-value pairs, appending to arrays, and exporting the final result
/// to a file.
pub struct JsonHandler {
    /// The JSON data being constructed.
    pub(crate) data: Value,
    /// Optional path where the JSON data will be exported.
    export_path: Option<PathBuf>,
}

impl JsonHandler {
    /// Creates a new `JsonHandler` with an optional export path.
    /// If `export_path` is `None`, calls to `export()` will be no-ops.
    pub fn new(export_path: Option<PathBuf>) -> Self {
        Self { data: json!({}), export_path }
    }

    /// Adds or updates a key-value pair in the JSON object.
    /// If the key already exists, its value will be overwritten.
    pub fn add_item(&mut self, key: &str, value: Value) {
        self.data[key] = value;
    }

    /// Appends a value to the array at the specified key.
    /// Creates a new array if the key doesn't exist or is null.
    /// Panics if the key exists but is not an array or null.
    pub fn add_harness_detail(&mut self, key: &str, value: Value) {
        if self.data[key].is_null() {
            self.data[key] = json!([]);
        }
        self.data[key].as_array_mut().unwrap().push(value);
    }

    /// Exports the JSON data to the configured file path with pretty-printing.
    /// Returns an error if the file cannot be written.
    pub fn export(&self) -> Result<(), std::io::Error> {
        if let Some(path) = &self.export_path {
            std::fs::write(path, serde_json::to_string_pretty(&self.data)?)
        } else {
            Ok(())
        }
    }
}
