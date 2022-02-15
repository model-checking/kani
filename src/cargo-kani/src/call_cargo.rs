// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use toml::Value;

use crate::context::KaniContext;

impl KaniContext {
    /// Given a `file` (a .symtab.json), produce `{file}.out` by calling symtab2gb
    pub fn cargo_build(&self) -> Result<Vec<PathBuf>> {
        let flag_env = {
            let rustc_args = self.kani_rustc_flags();
            crate::util::join_osstring(&rustc_args, " ")
        };

        let build_target = env!("TARGET");
        let target_dir = self.args.target_dir.as_ref().unwrap_or(&PathBuf::from("target")).clone();
        let mut args: Vec<OsString> = Vec::new();

        if self.args.tests {
            args.push("test".into());
            args.push("--no-run".into());
        } else {
            args.push("build".into());
        }

        args.push("--target".into());
        args.push(build_target.into());

        args.push("--target-dir".into());
        args.push(target_dir.clone().into());

        if self.args.verbose {
            args.push("-v".into());
        }

        let mut cmd = Command::new("cargo");
        cmd.args(args)
            .env("RUSTC", &self.kani_rustc)
            .env("RUSTFLAGS", "--kani-flags")
            .env("KANIFLAGS", flag_env);

        if self.args.debug {
            cmd.env("KANI_LOG", "rustc_codegen_kani");
        }

        self.run_terminal(cmd)?;

        if self.args.dry_run {
            // mock an answer
            return Ok(vec![
                format!(
                    "{}/{}/debug/deps/dry-run.symtab.json",
                    target_dir.into_os_string().to_string_lossy(),
                    build_target
                )
                .into(),
            ]);
        }

        let build_glob = format!(
            "{}/{}/debug/deps/*.symtab.json",
            target_dir.into_os_string().to_string_lossy(),
            build_target
        );
        let results = glob::glob(&build_glob)?;

        // the logic to turn "Iter<Result<T, E>>" into "Result<Vec<T>, E>" doesn't play well
        // with anyhow, so a type annotation is required
        let symtabs: core::result::Result<Vec<PathBuf>, glob::GlobError> = results.collect();

        Ok(symtabs?)
    }
}

pub fn config_toml_to_args() -> Result<Vec<OsString>> {
    // TODO: `cargo locate-project` maybe?
    let file = std::fs::read_to_string("Cargo.toml");
    if let Ok(file) = file {
        toml_to_args(&file)
    } else {
        // Suppress the error if we can't find it, for now.
        Ok(vec![])
    }
}

/// Parse a config toml and extract the kani arguments we should try injecting
fn toml_to_args(file: &str) -> Result<Vec<OsString>> {
    let config = file.parse::<Value>()?;
    // We specifically rely on BTreeMap here to get a stable iteration order for unit testing
    let mut map: BTreeMap<String, Value> = BTreeMap::new();
    let keys = ["workspace.metadata.kani.flags", "package.metadata.kani.flags", "kani.flags"];

    for key in keys {
        if let Ok(val) = descend(&config, key) {
            if let Some(val) = val.as_table() {
                map.extend(val.iter().map(|(x, y)| (x.to_owned(), y.to_owned())));
            }
        }
    }

    let mut args = Vec::new();
    let mut suffixed_args = Vec::new();

    for (flag, value) in map {
        if flag == "cbmc-args" {
            insert_arg_from_toml(&flag, &value, &mut suffixed_args)?;
        } else {
            insert_arg_from_toml(&flag, &value, &mut args)?;
        }
    }

    args.extend(suffixed_args);

    Ok(args)
}

/// Translates one toml entry into arguments and inserts it into args
fn insert_arg_from_toml(flag: &str, value: &Value, args: &mut Vec<OsString>) -> Result<()> {
    match value {
        Value::Boolean(b) => {
            if *b {
                args.push(format!("--{}", flag).into());
            } else if flag.starts_with("no-") {
                // Seems iffy. Let's just not support this.
                bail!("{} disables a disabling flag. Just enable the flag instead.", flag);
            } else {
                args.push(format!("--no-{}", flag).into());
            }
        }
        Value::Array(a) => {
            args.push(format!("--{}", flag).into());
            for arg in a {
                if let Some(arg) = arg.as_str() {
                    args.push(arg.into());
                } else {
                    bail!("flag {} contains non-string values", flag);
                }
            }
        }
        Value::String(s) => {
            args.push(format!("--{}", flag).into());
            args.push(s.into());
        }
        _ => {
            bail!("Unknown key type {}", flag);
        }
    }
    Ok(())
}

/// Take 'a.b.c' and turn it into 'start['a']['b']['c']' reliably.
fn descend<'a>(start: &'a Value, table: &str) -> Result<&'a Value> {
    let mut current = start;
    for key in table.split('.') {
        let next = current.get(key);
        if next.is_none() {
            bail!("no key");
        }
        current = next.unwrap();
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_toml_parsing() {
        let a = "[workspace.metadata.kani]
                      flags = { default-checks = false, unwind = \"2\" }";
        let b = toml_to_args(a).unwrap();
        assert_eq!(b, vec!["--no-default-checks", "--unwind", "2"]);
    }
}
