// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-check-fail

// We currently assert(false) for reify function pointer.
// So this codegens but fails model checking.

use std::fmt;

struct OneOf {
    names: &'static [&'static str],
}

impl fmt::Display for OneOf {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "`{}{}`", self.bogus, self.names[0])
    }
}

fn main() {
    let v = OneOf { names: &["one"] };
    println!("{}", v);
}
