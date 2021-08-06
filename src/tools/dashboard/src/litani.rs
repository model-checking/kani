// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Utilities to interact with the `Litani` build accumulator.

use pulldown_cmark::escape::StrWrite;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command};

/// Data structure representing a `Litani` build.
pub struct Litani {
    /// A buffer of the `spawn`ed Litani jobs so far. `Litani` takes some time
    /// to execute each `add-job` command and executing thousands of them
    /// sequentially takes a considerable amount of time. To speed up the
    /// execution of those commands, we spawn those commands sequentially (as
    /// normal). However, instead of `wait`ing for each process to terminate,
    /// we add its handle to a buffer of the `spawn`ed processes and continue
    /// with our program. Once we are done adding jobs, we wait for all of them
    /// to terminate before we run the `run-build` command.
    spawned_commands: Vec<Child>,
}

impl Litani {
    /// Sets up a new [`Litani`] run.
    pub fn init(project_name: &str, output_prefix: &Path, output_symlink: &Path) -> Self {
        Command::new("litani")
            .args([
                "init",
                "--project-name",
                project_name,
                "--output-prefix",
                output_prefix.to_str().unwrap(),
                "--output-symlink",
                output_symlink.to_str().unwrap(),
            ])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        Self { spawned_commands: Vec::new() }
    }

    /// Adds a single command with its dependencies.
    pub fn add_job(
        &mut self,
        command: &Command,
        inputs: &[&Path],
        outputs: &[&Path],
        description: &str,
        pipeline: &str,
        stage: &str,
        exit_status: i32,
        timeout: u32,
    ) {
        let mut job = Command::new("litani");
        // The given command may contain additional env vars. Prepend those vars
        // to the command before passing it to Litani.
        let job_envs: HashMap<_, _> = job.get_envs().collect();
        let mut new_envs = String::new();
        command.get_envs().fold(&mut new_envs, |fmt, (k, v)| {
            if !job_envs.contains_key(k) {
                fmt.write_fmt(format_args!(
                    "{}=\"{}\" ",
                    k.to_str().unwrap(),
                    v.unwrap().to_str().unwrap()
                ))
                .unwrap();
            }
            fmt
        });
        job.args([
            "add-job",
            "--command",
            &format!("{}{:?}", new_envs, command),
            "--description",
            description,
            "--pipeline-name",
            pipeline,
            "--ci-stage",
            stage,
            "--ok-returns",
            &exit_status.to_string(),
            "--timeout",
            &timeout.to_string(),
        ]);
        if !inputs.is_empty() {
            job.arg("--inputs").args(inputs);
        }
        if !outputs.is_empty() {
            job.arg("--outputs").args(outputs).arg("--phony-outputs").args(outputs);
        }
        // Start executing the command, but do not wait for it to terminate.
        self.spawned_commands.push(job.spawn().unwrap());
    }

    /// Starts a [`Litani`] run.
    pub fn run_build(&mut self) {
        // Wait for all spawned processes to terminate.
        for command in self.spawned_commands.iter_mut() {
            command.wait().unwrap();
        }
        self.spawned_commands.clear();
        // Run `run-build` command and wait for it to finish.
        Command::new("litani").arg("run-build").spawn().unwrap().wait().unwrap();
    }
}
