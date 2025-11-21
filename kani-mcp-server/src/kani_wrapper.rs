use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info, warn};

/// Configuration options for running Kani verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KaniOptions {
    pub path: PathBuf,
    pub harness: Option<String>,
    pub tests: bool,
    pub output_format: String,
    pub enable_unstable: Vec<String>,
    pub extra_args: Vec<String>,
    pub concrete_playback: bool,
    pub coverage: bool,
}

impl Default for KaniOptions {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            harness: None,
            tests: false,
            output_format: "terse".to_string(),
            enable_unstable: vec![],
            extra_args: vec![],
            concrete_playback: false,
            coverage: false,
        }
    }
}

/// Result of a Kani verification run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub success: bool,
    pub summary: String,
    pub harnesses: Vec<HarnessResult>,
    pub failed_checks: Vec<FailedCheck>,
    pub verification_time: Option<f64>,
    pub raw_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessResult {
    pub name: String,
    pub status: String,
    pub checks_passed: u32,
    pub checks_failed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedCheck {
    pub description: String,
    pub file: String,
    pub line: Option<u32>,
    pub function: String,
}

/// Wrapper around cargo-kani for executing verification
pub struct KaniWrapper {}

impl KaniWrapper {
    /// Create a new KaniWrapper and verify cargo-kani is available and reuses kani-driver's setup_cargo_command functionality
    pub fn new() -> Result<Self> {
        let mut cmd = Self::setup_cargo_command()?;
        cmd.arg("kani").arg("--version");

        let output = cmd.output()
            .context("cargo-kani not found in PATH. Please install Kani:\n  cargo install --locked kani-verifier\n  cargo kani setup")?;

        if !output.status.success() {
            anyhow::bail!("cargo-kani not properly installed or setup failed");
        }

        info!("✓ Found cargo-kani");
        Ok(Self {})
    }

    fn setup_cargo_command() -> Result<Command> {
        let cmd = Command::new("cargo");
        Ok(cmd)
    }

    /// Run Kani verification with the given options
    pub async fn verify(&self, options: KaniOptions) -> Result<VerificationResult> {
        info!("Starting Kani verification on: {:?}", options.path);

        if !options.path.exists() {
            anyhow::bail!("Path does not exist: {:?}", options.path);
        }

        let mut cmd = Self::setup_cargo_command()?;
        cmd.arg("kani");
        cmd.current_dir(&options.path);

        if let Some(harness) = &options.harness {
            cmd.arg("--harness").arg(harness);
            info!("  Filtering to harness: {}", harness);
        }

        if options.tests {
            cmd.arg("--tests");
            info!("  Running all tests as harnesses");
        }

        if !options.output_format.is_empty() {
            cmd.arg(format!("--output-format={}", options.output_format));
        }

        for feature in &options.enable_unstable {
            cmd.arg("--enable-unstable").arg(feature);
        }

        if options.concrete_playback {
            cmd.arg("-Z").arg("concrete-playback");
            cmd.arg("--concrete-playback=print");
        }

        if options.coverage {
            cmd.arg("--coverage");
        }

        for arg in &options.extra_args {
            cmd.arg(arg);
        }

        debug!("Executing command: {:?}", cmd);

        // Execute and capture output
        let start = std::time::Instant::now();
        let output = cmd.output().context("Failed to execute cargo-kani. Is it installed?")?;
        let duration = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined_output = format!("{}\n{}", stdout, stderr);

        debug!("Kani completed in {:.2}s", duration.as_secs_f64());

        if !stderr.is_empty() && stderr.contains("error") {
            warn!("Kani stderr: {}", stderr);
        }

        let result = self.parse_output(&combined_output, output.status.success())?;

        info!("Verification complete: {}", result.summary);

        Ok(result)
    }

    /// Parse Kani output into structured result
    fn parse_output(&self, output: &str, success: bool) -> Result<VerificationResult> {
        use crate::parser::KaniOutputParser;

        let parser = KaniOutputParser::new(output);
        let harnesses = parser.parse_harnesses();
        let failed_checks = parser.parse_failed_checks();
        let verification_time = parser.parse_verification_time();

        let total_harnesses = harnesses.len();
        let failed_harnesses = harnesses.iter().filter(|h| h.status == "FAILED").count();

        let summary = if success {
            format!("Verification successful! {} harness(es) verified.", total_harnesses)
        } else {
            format!(
                "Verification failed. {}/{} harness(es) failed with {} check failure(s).",
                failed_harnesses,
                total_harnesses,
                failed_checks.len()
            )
        };

        Ok(VerificationResult {
            success,
            summary,
            harnesses,
            failed_checks,
            verification_time,
            raw_output: output.to_string(),
        })
    }
}
