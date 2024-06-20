// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This harness was based on firecracker argument parsing code from arg_parser.rs in
//! firecracker/src/utils/src. It used to get stuck in post-processing with unwind of two or more.
use std::collections::BTreeMap;

pub struct ArgParser<'a> {
    arguments: BTreeMap<&'a str, ()>,
}

impl<'a> ArgParser<'a> {
    fn format_arguments(&self) -> String {
        self.arguments
            .values()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|_arg| String::new())
            .collect::<Vec<_>>()
            .join("")
    }
}

#[kani::proof]
#[kani::unwind(2)]
fn main() {
    let a: ArgParser = ArgParser { arguments: BTreeMap::new() };
    a.format_arguments();
}
