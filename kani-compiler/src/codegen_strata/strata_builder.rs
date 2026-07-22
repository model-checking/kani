// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Strata IR builder - constructs Strata Core dialect programs

use std::fmt::Write;

/// Builder for Strata Core dialect programs
pub struct StrataBuilder {
    output: String,
}

impl StrataBuilder {
    pub fn new() -> Self {
        Self {
            output: String::from("program Core;\n\n"),
        }
    }

    pub fn add_global_var(&mut self, name: &str, ty: &str) {
        writeln!(self.output, "var {} : {};", name, ty).unwrap();
    }

    pub fn add_procedure(&mut self, name: &str, params: &[(String, String)], returns: &[(String, String)], body: &str) {
        write!(self.output, "procedure {}(", name).unwrap();
        for (i, (pname, pty)) in params.iter().enumerate() {
            if i > 0 { write!(self.output, ", ").unwrap(); }
            write!(self.output, "{} : {}", pname, pty).unwrap();
        }
        write!(self.output, ")").unwrap();

        if !returns.is_empty() {
            write!(self.output, " returns (").unwrap();
            for (i, (rname, rty)) in returns.iter().enumerate() {
                if i > 0 { write!(self.output, ", ").unwrap(); }
                write!(self.output, "{} : {}", rname, rty).unwrap();
            }
            write!(self.output, ")").unwrap();
        }

        writeln!(self.output, "\n{{\n{}\n}}\n", body).unwrap();
    }

    pub fn build(self) -> String {
        self.output
    }
}
