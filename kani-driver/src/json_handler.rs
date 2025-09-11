use serde_json::{json, Value};
use std::path::PathBuf;

pub struct JsonHandler {
    pub(crate) data: Value,
    export_path: Option<PathBuf>,
}

impl JsonHandler {
    pub fn new(export_path: Option<PathBuf>) -> Self {
        Self {
            data: json!({}),
            export_path,
        }
    }

    pub fn add_item(&mut self, key: &str, value: Value) {
        self.data[key] = value;
    }

    pub fn add_harness_detail(&mut self, key: &str, value: Value) {
        if self.data[key].is_null() {
            self.data[key] = json!([]);
        }
        self.data[key].as_array_mut().unwrap().push(value);
    }

    pub fn export(&self) -> Result<(), std::io::Error> {
        if let Some(path) = &self.export_path {
            std::fs::write(path, serde_json::to_string_pretty(&self.data)?)
        } else {
            Ok(())
        }
    }
}
