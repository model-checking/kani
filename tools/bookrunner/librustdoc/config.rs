// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use std::convert::TryFrom;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OutputFormat {
    Json,
    Html,
}

impl Default for OutputFormat {
    fn default() -> OutputFormat {
        OutputFormat::Html
    }
}

impl OutputFormat {}

impl TryFrom<&str> for OutputFormat {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "json" => Ok(OutputFormat::Json),
            "html" => Ok(OutputFormat::Html),
            _ => Err(format!("unknown output format `{}`", value)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum EmitType {
    Unversioned,
    Toolchain,
    InvocationSpecific,
}

impl FromStr for EmitType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use EmitType::*;
        match s {
            "unversioned-shared-resources" => Ok(Unversioned),
            "toolchain-shared-resources" => Ok(Toolchain),
            "invocation-specific" => Ok(InvocationSpecific),
            _ => Err(()),
        }
    }
}
