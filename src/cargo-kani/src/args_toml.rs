// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use toml::value::Table;
use toml::Value;

/// Produces a set of arguments to pass to ourself (cargo-kani) from a Cargo.toml project file
pub fn config_toml_to_args() -> Result<Vec<OsString>> {
    let file = std::fs::read_to_string(cargo_locate_project()?)?;
    toml_to_args(&file)
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

/// Parse a config toml string and extract the cargo-kani arguments we should try injecting
fn toml_to_args(tomldata: &str) -> Result<Vec<OsString>> {
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
    let mut suffixed_args = Vec::new();

    for (flag, value) in map {
        if flag == "cbmc-args" {
            // --cbmc-args has to come last because it eats all remaining arguments
            insert_arg_from_toml(&flag, &value, &mut suffixed_args)?;
        } else {
            insert_arg_from_toml(&flag, &value, &mut args)?;
        }
    }

    args.extend(suffixed_args);

    Ok(args)
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
        assert_eq!(b, vec!["--no-default-checks", "--unwind", "2", "--cbmc-args", "--fake"]);
    }
}
