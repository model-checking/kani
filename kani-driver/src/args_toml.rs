// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use toml::value::Table;
use toml::Value;

/// Produce the list of arguments to pass to ourself (cargo-kani).
///
/// The arguments passed via command line have precedence over the ones from the Cargo.toml.
pub fn join_args(input_args: Vec<OsString>) -> Result<Vec<OsString>> {
    let file = std::fs::read_to_string(cargo_locate_project()?)?;
    let (kani_args, cbmc_args) = toml_to_args(&file)?;
    merge_args(input_args, kani_args, cbmc_args)
}

/// Join the arguments passed via command line with the ones found in the Cargo.toml.
///
/// The arguments passed via command line have precedence over the ones from the Cargo.toml. Thus,
/// we need to inject the command line arguments after the ones read from Cargo.toml. This can be
/// a bit annoying given that cbmc args have to be at the end of the arguments and the "--cbmc-args"
/// flag must only be included once.
///
/// This function will return the arguments in the following order:
/// ```
/// <bin_name> [<cfg_kani_args>]* [<cmd_kani_args>]* [--cbmc-args [<cfg_cbmc_args>]* [<cmd_cbmc_args>]*]
/// ```
fn merge_args(
    cmd_args: Vec<OsString>,
    cfg_kani_args: Vec<OsString>,
    cfg_cbmc_args: Vec<OsString>,
) -> Result<Vec<OsString>> {
    let mut merged_args =
        vec![cmd_args.first().expect("Expected binary path as one argument").clone()];
    merged_args.extend(cfg_kani_args);
    if cfg_cbmc_args.is_empty() {
        // No need to handle cbmc_args. Just merge the Cargo.toml args with the original input:
        // [<config_kani_args>]* [input_args]*
        merged_args.extend_from_slice(&cmd_args[1..]);
    } else {
        let cbmc_flag = cmd_args.iter().enumerate().find(|&f| f.1 == "--cbmc-args");
        if let Some((idx, _)) = cbmc_flag {
            // Both command line and config file have --cbmc-args. Merge them to be in order.
            merged_args.extend_from_slice(&cmd_args[1..idx]);
            merged_args.extend(cfg_cbmc_args);
            // Remove --cbmc-args from the input.
            merged_args.extend_from_slice(&cmd_args[idx + 1..]);
        } else {
            // Command line doesn't have --cbmc-args. Put command line arguments in the middle.
            // [<cfg_kani_args>]* [<cmd_args>]* --cbmc-args [<cfg_cbmc_args>]+
            merged_args.extend_from_slice(&cmd_args[1..]);
            merged_args.extend(cfg_cbmc_args);
        }
    }
    Ok(merged_args)
}

/// `locate-project` produces a response like: `/full/path/to/src/cargo-kani/Cargo.toml`
fn cargo_locate_project() -> Result<PathBuf> {
    let cmd =
        Command::new("cargo").args(["locate-project", "--message-format", "plain"]).output()?;
    if !cmd.status.success() {
        let err = std::str::from_utf8(&cmd.stderr)?;
        bail!("{}", err);
    }
    let path = std::str::from_utf8(&cmd.stdout)?;
    // A trim is essential: remove the trailing newline
    Ok(path.trim().into())
}

/// Parse a config toml string and extract the cargo-kani arguments we should try injecting.
/// This returns two different vectors since all cbmc-args have to be at the end.
fn toml_to_args(tomldata: &str) -> Result<(Vec<OsString>, Vec<OsString>)> {
    let config = tomldata.parse::<Value>()?;
    // To make testing easier, our function contract is to produce a stable ordering of flags for a given input.
    // Consequently, we use BTreeMap instead of HashMap here.
    let mut map: BTreeMap<String, Value> = BTreeMap::new();
    let tables = ["workspace.metadata.kani.flags", "package.metadata.kani.flags", "kani.flags"];

    for table in tables {
        if let Some(val) = get_table(&config, table) {
            map.extend(val.iter().map(|(x, y)| (x.to_owned(), y.to_owned())));
        }
    }

    let mut args = Vec::new();
    let mut cbmc_args = Vec::new();

    for (flag, value) in map {
        if flag == "cbmc-args" {
            // --cbmc-args has to come last because it eats all remaining arguments
            insert_arg_from_toml(&flag, &value, &mut cbmc_args)?;
        } else {
            insert_arg_from_toml(&flag, &value, &mut args)?;
        }
    }

    Ok((args, cbmc_args))
}

/// Translates one toml entry (flag, value) into arguments and inserts it into `args`
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

/// Take 'a.b.c' and turn it into 'start['a']['b']['c']' reliably, and interpret the result as a table
fn get_table<'a>(start: &'a Value, table: &str) -> Option<&'a Table> {
    let mut current = start;
    for key in table.split('.') {
        current = current.get(key)?;
    }
    current.as_table()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_toml_parsing() {
        let a = "[workspace.metadata.kani]
                      flags = { default-checks = false, unwind = \"2\", cbmc-args = [\"--fake\"] }";
        let b = toml_to_args(a).unwrap();
        // default first, then unwind thanks to btree ordering.
        // cbmc-args always last.
        assert_eq!(b.0, vec!["--no-default-checks", "--unwind", "2"]);
        assert_eq!(b.1, vec!["--cbmc-args", "--fake"]);
    }

    #[test]
    fn check_merge_args_with_only_command_line_args() {
        let cmd_args: Vec<OsString> =
            ["cargo_kani", "--no-default-checks", "--unwind", "2", "--cbmc-args", "--fake"]
                .iter()
                .map(|&s| s.into())
                .collect();
        let merged = merge_args(cmd_args.clone(), Vec::new(), Vec::new()).unwrap();
        assert_eq!(merged, cmd_args);
    }

    #[test]
    fn check_merge_args_with_only_config_kani_args() {
        let cfg_args: Vec<OsString> =
            ["--no-default-checks", "--unwind", "2"].iter().map(|&s| s.into()).collect();
        let merged = merge_args(vec!["kani".into()], cfg_args.clone(), Vec::new()).unwrap();
        assert_eq!(merged[0], OsString::from("kani"));
        assert_eq!(merged[1..], cfg_args);
    }

    #[test]
    fn check_merge_args_order() {
        let cmd_args: Vec<OsString> =
            vec!["kani".into(), "--debug".into(), "--cbmc-args".into(), "--fake".into()];
        let cfg_kani_args: Vec<OsString> = vec!["--no-default-checks".into()];
        let cfg_cbmc_args: Vec<OsString> = vec!["--cbmc-args".into(), "--trace".into()];
        let merged =
            merge_args(cmd_args.clone(), cfg_kani_args.clone(), cfg_cbmc_args.clone()).unwrap();
        assert_eq!(merged.len(), cmd_args.len() + cfg_kani_args.len() + cfg_cbmc_args.len() - 1);
        assert_eq!(merged[0], OsString::from("kani"));
        assert_eq!(merged[1], OsString::from("--no-default-checks"));
        assert_eq!(merged[2], OsString::from("--debug"));
        assert_eq!(merged[3], OsString::from("--cbmc-args"));
        assert_eq!(merged[4], OsString::from("--trace"));
        assert_eq!(merged[5], OsString::from("--fake"));
    }
}
