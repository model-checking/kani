// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

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
            if line.contains("Checking harness")
                && let Some(name_part) = line.split("Checking harness").nth(1)
            {
                current_harness = Some(name_part.trim().trim_end_matches("...").to_string());
            }

            // Detect harness completion
            if line.contains("VERIFICATION")
                && let Some(name) = current_harness.take()
            {
                let status = if line.contains("SUCCESSFUL") {
                    "SUCCESS"
                } else if line.contains("FAILED") {
                    "FAILED"
                } else {
                    "UNKNOWN"
                }
                .to_string();

                harnesses.push(HarnessResult { name, status, checks_passed: 0, checks_failed: 0 });
            }
        }

        harnesses
    }

    /// Parse failed checks from output with detailed information
    pub fn parse_failed_checks(&self) -> Vec<FailedCheck> {
        let mut failed_checks = Vec::new();

        for line in self.output.lines() {
            if line.contains("Check ") && line.contains(":") {
                let check_info = self.parse_single_check(line);
                if let Some(check) = check_info {
                    failed_checks.push(check);
                }
            }
        }

        // Also parse from "Failed Checks:" section
        let mut in_failed_section = false;
        for line in self.output.lines() {
            if line.contains("Failed Checks:") {
                in_failed_section = true;
                continue;
            }

            if in_failed_section {
                if line.trim().is_empty() || line.starts_with("VERIFICATION") {
                    break;
                }

                if let Some(check) = self.parse_failed_check_line(line)
                    && !failed_checks
                        .iter()
                        .any(|c| c.description == check.description && c.line == check.line)
                {
                    failed_checks.push(check);
                }
            }
        }

        failed_checks
    }

    /// Parse a single check result line
    fn parse_single_check(&self, line: &str) -> Option<FailedCheck> {
        let _check_num = line.split("Check").nth(1)?.split(':').next()?.trim();

        // Look for the next few lines to get details
        let lines: Vec<&str> = self.output.lines().collect();
        let current_idx = lines.iter().position(|l| *l == line)?;

        let mut description = String::new();
        let mut file = String::new();
        let mut line_num = None;
        let mut function = String::new();
        let mut is_failure = false;

        // Parse the next few lines for details
        for detail_line in lines.iter().skip(current_idx + 1).take(4) {
            if detail_line.contains("- Status: FAILURE") {
                is_failure = true;
            }

            if detail_line.contains("- Description:") {
                description = detail_line
                    .split("- Description:")
                    .nth(1)?
                    .trim()
                    .trim_matches('"')
                    .to_string();
            }

            if detail_line.contains("- Location:") {
                let location = detail_line.split("- Location:").nth(1)?.trim();
                let parts: Vec<&str> = location.split(" in function ").collect();

                if let Some(file_part) = parts.first() {
                    let file_line: Vec<&str> = file_part.split(':').collect();
                    file = file_line.first()?.trim_start_matches("./").to_string();
                    if let Some(ln) = file_line.get(1) {
                        line_num = ln.parse().ok();
                    }
                }

                if let Some(func_part) = parts.get(1) {
                    function = func_part.trim().to_string();
                }
            }
        }

        if is_failure {
            Some(FailedCheck { description, file, line: line_num, function })
        } else {
            None
        }
    }

    /// Parse a line from the "Failed Checks:" section
    fn parse_failed_check_line(&self, line: &str) -> Option<FailedCheck> {
        let description = line.split("File:").next()?.trim().to_string();

        if let Some(file_part) = line.split("File:").nth(1) {
            let parts: Vec<&str> = file_part.split(',').collect();

            let file = parts.first()?.trim().trim_matches('"').trim_start_matches("./").to_string();

            let line_num = parts
                .get(1)
                .and_then(|s| s.split_whitespace().nth(1))
                .and_then(|s| s.parse::<u32>().ok());

            let function = parts
                .get(2)
                .and_then(|s| s.split("in").nth(1))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            return Some(FailedCheck { description, file, line: line_num, function });
        }

        None
    }

    /// Parse verification time from output
    pub fn parse_verification_time(&self) -> Option<f64> {
        for line in self.output.lines() {
            if line.contains("Verification Time:")
                && let Some(time_str) = line.split(':').nth(1)
            {
                return time_str.trim().trim_end_matches('s').parse::<f64>().ok();
            }
        }
        None
    }

    /// Extract counterexamples from output
    pub fn parse_counterexamples(&self) -> Vec<String> {
        let mut counterexamples = Vec::new();

        for line in self.output.lines() {
            if line.contains("SAT checker: instance is SATISFIABLE") {
                counterexamples.push(
                    "Counterexample found (inputs exist that violate the property)".to_string(),
                );
            }

            if line.contains("Violated property:") || line.contains("Counterexample:") {
                counterexamples.push(line.trim().to_string());
            }
        }

        counterexamples
    }

    /// Extract code context from output
    pub fn extract_code_context(&self) -> Option<String> {
        for line in self.output.lines() {
            if line.contains("-->") && line.contains(".rs:") {
                return Some(line.trim().to_string());
            }
        }
        None
    }

    /// Generate detailed failure explanation
    pub fn generate_detailed_explanation(&self) -> String {
        let failed_checks = self.parse_failed_checks();
        let counterexamples = self.parse_counterexamples();
        let harnesses = self.parse_harnesses();
        let verification_time = self.parse_verification_time();

        let mut explanation = String::new();

        explanation.push_str("DETAILED KANI VERIFICATION FAILURE ANALYSIS\n");
        explanation.push_str("═══════════════════════════════════════════════\n\n");

        // Summary
        explanation.push_str("Summary:\n");
        explanation.push_str(&format!("  • Total harnesses: {}\n", harnesses.len()));
        explanation.push_str(&format!("  • Failed checks: {}\n", failed_checks.len()));
        if let Some(time) = verification_time {
            explanation.push_str(&format!("  • Verification time: {:.3}s\n", time));
        }
        explanation.push('\n');

        // Failed checks detail
        if !failed_checks.is_empty() {
            explanation.push_str("FAILED CHECKS:\n\n");

            for (i, check) in failed_checks.iter().enumerate() {
                explanation.push_str(&format!("{}. {}\n", i + 1, check.description));
                explanation.push_str(&format!(
                    "   Location: {}:{}\n",
                    check.file,
                    check.line.map(|l| l.to_string()).unwrap_or_else(|| "?".to_string())
                ));
                explanation.push_str(&format!("   Function: {}\n", check.function));
                explanation.push('\n');
            }
        }

        // Counterexamples
        if !counterexamples.is_empty() {
            explanation.push_str("COUNTEREXAMPLES:\n\n");
            for ce in counterexamples {
                explanation.push_str(&format!("  • {}\n", ce));
            }
            explanation.push('\n');
        }

        // Root cause analysis
        explanation.push_str("ROOT CAUSE ANALYSIS:\n\n");
        if !failed_checks.is_empty() {
            let first_check = &failed_checks[0];

            explanation.push_str(&format!(
                "The assertion '{}' failed, which indicates:\n",
                first_check.description
            ));

            // Pattern-based analysis
            if first_check.description.contains("overflow") {
                explanation.push_str("  • An arithmetic overflow occurred\n");
                explanation.push_str("  • The operation exceeded the maximum value for the type\n");
                explanation.push_str("  • This happens with certain input combinations\n");
            } else if first_check.description.contains("panic") {
                explanation.push_str("  • The code panicked during execution\n");
                explanation.push_str("  • An unhandled error condition was reached\n");
            } else if first_check.description.contains("assertion") {
                explanation.push_str("  • A programmer-defined assertion failed\n");
                explanation.push_str("  • The expected property does not hold for all inputs\n");
            } else if first_check.description.contains("dereference") {
                explanation.push_str("  • An invalid pointer dereference occurred\n");
                explanation.push_str("  • Accessing memory that shouldn't be accessed\n");
            } else if first_check.description.contains("index")
                || first_check.description.contains("bounds")
            {
                explanation.push_str("  • An array/slice index is out of bounds\n");
                explanation.push_str("  • Accessing beyond the valid range of the collection\n");
            } else {
                explanation.push_str("  • A safety or correctness property was violated\n");
                explanation
                    .push_str("  • The code doesn't handle all possible input cases correctly\n");
            }
            explanation.push('\n');
        }

        // Suggested fixes
        explanation.push_str("SUGGESTED FIXES:\n\n");
        if !failed_checks.is_empty() {
            let first_check = &failed_checks[0];

            if first_check.description.contains("overflow") {
                explanation.push_str("  1. Use checked arithmetic operations:\n");
                explanation.push_str("     • Replace `a + b` with `a.checked_add(b)`\n");
                explanation.push_str("     • Handle the `None` case appropriately\n");
                explanation.push_str("  2. Or use saturating arithmetic:\n");
                explanation.push_str("     • Replace with `a.saturating_add(b)`\n");
                explanation.push_str("  3. Add input validation:\n");
                explanation.push_str("     • Use `kani::assume()` to constrain inputs\n");
            } else if first_check.description.contains("dereference")
                || first_check.description.contains("null")
            {
                explanation.push_str("  1. Add null pointer checks:\n");
                explanation.push_str("     • Check `if ptr.is_null()` before dereferencing\n");
                explanation.push_str("  2. Use safe Rust alternatives:\n");
                explanation
                    .push_str("     • Consider using `Option<&T>` instead of raw pointers\n");
            } else if first_check.description.contains("index")
                || first_check.description.contains("bounds")
            {
                explanation.push_str("  1. Add bounds checking:\n");
                explanation.push_str("     • Use `.get()` instead of direct indexing\n");
                explanation.push_str("     • Check `index < array.len()` before access\n");
                explanation.push_str("  2. Use iterators:\n");
                explanation
                    .push_str("     • Consider using `.iter()` instead of manual indexing\n");
            } else {
                explanation.push_str("  1. Review the assertion condition:\n");
                explanation.push_str("     • Ensure it correctly captures the intended property\n");
                explanation.push_str("  2. Add input constraints:\n");
                explanation.push_str("     • Use `kani::assume()` to limit the input space\n");
                explanation.push_str("  3. Fix the underlying logic:\n");
                explanation.push_str("     • Adjust the code to handle all cases correctly\n");
            }
            explanation.push('\n');
        }

        // Next steps
        explanation.push_str("NEXT STEPS:\n\n");
        explanation.push_str("  1. Examine the code at the failure location\n");
        explanation.push_str("  2. Understand what inputs trigger the failure\n");
        explanation.push_str("  3. Apply the suggested fixes\n");
        explanation.push_str("  4. Re-run Kani verification to confirm the fix\n");
        explanation.push_str("  5. Consider adding more proof harnesses for edge cases\n");

        explanation
    }
}
