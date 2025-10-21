use crate::kani_wrapper::{FailedCheck, HarnessResult};

/// Parser for Kani output format
pub struct KaniOutputParser<'a> {
    output: &'a str,
}

impl<'a> KaniOutputParser<'a> {
    pub fn new(output: &'a str) -> Self {
        Self { output }
    }

    /// Parse harness results from output
    pub fn parse_harnesses(&self) -> Vec<HarnessResult> {
        let mut harnesses = Vec::new();
        let mut current_harness: Option<String> = None;

        for line in self.output.lines() {
            // Detect harness start: "Checking harness module::function..."
            if line.contains("Checking harness") {
                if let Some(name_part) = line.split("Checking harness").nth(1) {
                    current_harness = Some(name_part.trim().trim_end_matches("...").to_string());
                }
            }

            // Detect harness completion
            if line.contains("VERIFICATION") {
                if let Some(name) = current_harness.take() {
                    let status = if line.contains("SUCCESSFUL") {
                        "SUCCESS"
                    } else if line.contains("FAILED") {
                        "FAILED"
                    } else {
                        "UNKNOWN"
                    }.to_string();

                    harnesses.push(HarnessResult {
                        name,
                        status,
                        checks_passed: 0,
                        checks_failed: 0,
                    });
                }
            }
        }

        harnesses
    }

    /// Parse failed checks from output
    pub fn parse_failed_checks(&self) -> Vec<FailedCheck> {
        let mut failed_checks = Vec::new();
        let mut in_failed_section = false;

        for line in self.output.lines() {
            if line.contains("Failed Checks:") {
                in_failed_section = true;
                continue;
            }

            if in_failed_section {
                // Stop at empty line or next section
                if line.trim().is_empty() || line.starts_with("VERIFICATION") {
                    in_failed_section = false;
                    continue;
                }

                // Parse failed check line
                // Format: "description File: "path", line X, in function"
                let description = line.split("File:").next()
                    .unwrap_or(line)
                    .trim()
                    .to_string();

                let (file, line_num, function) = if let Some(file_part) = line.split("File:").nth(1) {
                    let parts: Vec<&str> = file_part.split(',').collect();
                    let file = parts.get(0)
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    
                    let line_num = parts.get(1)
                        .and_then(|s| s.split_whitespace().nth(1))
                        .and_then(|s| s.parse::<u32>().ok());
                    
                    let function = parts.get(2)
                        .and_then(|s| s.split("in").nth(1))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    (file, line_num, function)
                } else {
                    ("unknown".to_string(), None, "unknown".to_string())
                };

                failed_checks.push(FailedCheck {
                    description,
                    file,
                    line: line_num,
                    function,
                });
            }
        }

        failed_checks
    }

    /// Parse verification time from output
    pub fn parse_verification_time(&self) -> Option<f64> {
        for line in self.output.lines() {
            if line.contains("Verification Time:") {
                if let Some(time_str) = line.split(':').nth(1) {
                    return time_str.trim()
                        .trim_end_matches('s')
                        .parse::<f64>()
                        .ok();
                }
            }
        }
        None
    }
}