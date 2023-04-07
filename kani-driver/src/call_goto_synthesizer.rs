// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::util::warning;
use anyhow::Result;
use kani_metadata::HarnessMetadata;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::session::KaniSession;

impl KaniSession {
    /// Synthesize loop contracts for a goto binary `input` and produce a new goto binary `output`
    /// The synthesizer we use is `goto-synthesizer` built in CBMC codebase, which is an enumerative
    /// loop-contracts synthesizer. `goto-synthesizer` enumerates and checks if a candidate can be
    /// used to prove some assertions, and applies found invariants when all checks pass.
    pub fn synthesize_loop_contracts(
        &self,
        input: &Path,
        output: &Path,
        harness_metadata: &HarnessMetadata,
    ) -> Result<()> {
        if !self.args.quiet {
            println!("Running loop contract synthesizer.");
            warning("This process may not terminate.");
            warning(
                "Loop-contracts synthesizer is not compatible with unwinding bounds. Unwind bounds will be ignored.",
            );
        }

        let mut args: Vec<OsString> = vec![
            "--loop-contracts-no-unwind".into(),
            input.to_owned().into_os_string(),  // input
            output.to_owned().into_os_string(), // output
        ];

        self.handle_solver_args(&harness_metadata.attributes.solver, &mut args)?;

        let mut cmd = Command::new("goto-synthesizer");
        cmd.args(args);

        self.run_suppress(cmd)?;

        Ok(())
    }
}
