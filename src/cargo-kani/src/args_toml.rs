// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::ffi::OsString;
use toml::Value;

/// Produces a set of arguments to pass to "self" (cargo-kani) from a Cargo.toml project file
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

/// Parse a config toml and extract the cargo-kani arguments we should try injecting
fn toml_to_args(file: &str) -> Result<Vec<OsString>> {
    let config = file.parse::<Value>()?;
    // We specifically rely on BTreeMap here to get a stable iteration order for unit testing
    let mut map: BTreeMap<String, Value> = BTreeMap::new();
    let keys = ["workspace.metadata.kani.flags", "package.metadata.kani.flags", "kani.flags"];

    for key in keys {
        if let Some(val) = descend(&config, key) {
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
fn descend<'a>(start: &'a Value, table: &str) -> Option<&'a Value> {
    let mut current = start;
    for key in table.split('.') {
        current = current.get(key)?;
    }
    Some(current)
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
