// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A writer for Boogie programs.
//! Generates a text Boogie program with the following format:
//! ```
//! // Type declarations:
//! <Type declaration 1>
//! <Type declaration 2>
//! ...
//!
//! // Constant declarations:
//! <Const declaration 1>
//! <Const declaration 2>
//! ...
//!
//! // Variable declarations:
//! var <var-name1>: <type1>;
//! var <var-name2>: <type2>;
//! ...
//!
//! // Axioms
//! axiom <expr1>;
//! axiom <expr2>;
//! ...
//!
//! // Functions:
//! function <function1-name>(<arg1>: <type1>, ...) returns (return-var-name: <return-type>)
//! {
//!   <body>
//! }
//! ...
//!
//! // Procedures:
//! procedure <procedure1-name>(<arg1>: <type1>, ...) returns (return-var-name: <return-type>)
//! requires <pre-condition1>;
//! requires <pre-condition2>;
//! ...
//! ensures <post-condition1>;
//! ensures <post-condition2>;
//! ...
//! modifies <var1>, <var2>, ...;
//! {
//!   <body>
//! }
//! ...
//!
///! ```
use crate::boogie_program::*;

use std::io::Write;

/// A writer for Boogie programs.
struct Writer<'a, T: Write> {
    writer: &'a mut T,
    indentation: usize,
}

/// A trait for objects (Boogie program constructs) that can be converted to
/// text (Boogie format)
trait Writable {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()>;
}

impl<'a, T: Write> Writer<'a, T> {
    fn new(writer: &'a mut T) -> Self {
        Self { writer, indentation: 0 }
    }

    /// forward to the `write_to` method of a Writable object
    fn write(&mut self, w: &impl Writable) -> std::io::Result<()> {
        w.write_to(self)
    }

    fn newline(&mut self) -> std::io::Result<()> {
        writeln!(self.writer)
    }

    fn increase_indent(&mut self) {
        self.indentation += 2;
    }

    fn decrease_indent(&mut self) {
        self.indentation -= 2;
    }

    fn indent(&mut self) -> std::io::Result<()> {
        write!(self.writer, "{:width$}", "", width = self.indentation)
    }
}

impl BoogieProgram {
    pub fn write_to<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        let mut writer = Writer::new(writer);
        writer.write(self)
    }
}

impl Writable for BoogieProgram {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        if !self.type_declarations.is_empty() {
            writeln!(writer.writer, "// Type declarations:")?;
            for _td in &self.type_declarations {
                todo!()
            }
        }
        if !self.const_declarations.is_empty() {
            writeln!(writer.writer, "// Constant declarations:")?;
            for _const_decl in &self.const_declarations {
                todo!()
            }
        }
        if !self.var_declarations.is_empty() {
            writeln!(writer.writer, "// Variable declarations:")?;
            for _var_decl in &self.var_declarations {
                todo!()
            }
        }
        if !self.axioms.is_empty() {
            writeln!(writer.writer, "// Axioms:")?;
            for _a in &self.axioms {
                todo!()
            }
        }
        if !self.functions.is_empty() {
            writeln!(writer.writer, "// Functions:")?;
            for _f in &self.functions {
                todo!()
            }
        }
        if !self.procedures.is_empty() {
            writeln!(writer.writer, "// Procedures:")?;
            for p in &self.procedures {
                writer.write(p)?;
            }
        }
        Ok(())
    }
}

impl Writable for Procedure {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        // signature
        write!(writer.writer, "procedure {}(", self.name)?;
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(writer.writer, ",")?;
            }
            write!(writer.writer, "{}: ", param.name)?;
            writer.write(&param.typ)?;
        }
        write!(writer.writer, ") ")?;
        if !self.return_type.is_empty() {
            write!(writer.writer, "returns (")?;
            for (i, (name, typ)) in self.return_type.iter().enumerate() {
                if i > 0 {
                    write!(writer.writer, ",")?;
                }
                write!(writer.writer, "{name}: ")?;
                writer.write(typ)?;
            }
            write!(writer.writer, ")")?;
        }
        writer.newline()?;

        // contract
        if let Some(contract) = &self.contract {
            writer.increase_indent();
            writer.write(contract)?;
            writer.decrease_indent();
        }
        writeln!(writer.writer, "{{")?;
        writer.increase_indent();
        writer.write(&self.body)?;
        writer.decrease_indent();
        writeln!(writer.writer, "}}")?;
        Ok(())
    }
}

impl Writable for Expr {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            Expr::Literal(value) => {
                writer.write(value)?;
            }
            Expr::Symbol { name } => {
                write!(writer.writer, "{name}")?;
            }
            Expr::UnaryOp { op, operand } => {
                writer.write(op)?;
                write!(writer.writer, "(")?;
                writer.write(operand.as_ref())?;
                write!(writer.writer, ")")?;
            }
            Expr::BinaryOp { op, left, right } => {
                write!(writer.writer, "(")?;
                writer.write(left.as_ref())?;
                write!(writer.writer, " ")?;
                writer.write(op)?;
                write!(writer.writer, " ")?;
                writer.write(right.as_ref())?;
                write!(writer.writer, ")")?;
            }
        }
        Ok(())
    }
}

impl Writable for Stmt {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            Stmt::Assignment { target, value } => {
                writer.indent()?;
                write!(writer.writer, "{} := ", target)?;
                writer.write(value)?;
                writeln!(writer.writer, ";")?;
            }
            Stmt::Assert { condition } => {
                writer.indent()?;
                write!(writer.writer, "assert ")?;
                writer.write(condition)?;
                writeln!(writer.writer, ";")?;
            }
            Stmt::Assume { condition } => {
                writer.indent()?;
                write!(writer.writer, "assume ")?;
                writer.write(condition)?;
                writeln!(writer.writer, ";")?;
            }
            Stmt::Block { statements } => {
                for s in statements {
                    writer.write(s)?;
                }
            }
            Stmt::Break => {
                writer.indent()?;
                writeln!(writer.writer, "break;")?;
            }
            Stmt::Call { symbol, arguments } => {
                writer.indent()?;
                write!(writer.writer, "{symbol}(")?;
                for (i, a) in arguments.iter().enumerate() {
                    if i > 0 {
                        write!(writer.writer, ", ")?;
                    }
                    writer.write(a)?;
                }
                writeln!(writer.writer, ");")?;
            }
            Stmt::Decl { name, typ } => {
                writer.indent()?;
                write!(writer.writer, "var {}: ", name)?;
                writer.write(typ)?;
                writeln!(writer.writer, ";")?;
            }
            Stmt::If { condition, body, else_body } => {
                writer.indent()?;
                write!(writer.writer, "if (")?;
                writer.write(condition)?;
                writeln!(writer.writer, ") {{")?;
                writer.increase_indent();
                writer.write(body.as_ref())?;
                writer.decrease_indent();
                writer.indent()?;
                write!(writer.writer, "}}")?;
                if let Some(else_body) = else_body {
                    writeln!(writer.writer, " else {{")?;
                    writer.increase_indent();
                    writer.write(else_body.as_ref())?;
                    writer.decrease_indent();
                    writer.indent()?;
                    write!(writer.writer, "}}")?;
                }
                writeln!(writer.writer)?;
            }
            Stmt::Goto { label } => {
                writer.indent()?;
                writeln!(writer.writer, "goto {label};")?;
            }
            Stmt::Label { label } => {
                writer.indent()?;
                writeln!(writer.writer, "{label}:")?;
            }
            Stmt::Return => {
                writer.indent()?;
                writeln!(writer.writer, "return;")?;
            }
            Stmt::While { condition, body } => {
                writer.indent()?;
                write!(writer.writer, "while (")?;
                writer.write(condition)?;
                writeln!(writer.writer, ") {{")?;
                writer.increase_indent();
                writer.write(body.as_ref())?;
                writer.decrease_indent();
                writeln!(writer.writer, "}}")?;
            }
        }
        Ok(())
    }
}

impl Writable for Contract {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        for r in &self.requires {
            writer.indent()?;
            write!(writer.writer, "requires ")?;
            writer.write(r)?;
            writeln!(writer.writer, ";")?;
        }
        for e in &self.ensures {
            writer.indent()?;
            write!(writer.writer, "ensures ")?;
            writer.write(e)?;
            writeln!(writer.writer, ";")?;
        }
        for m in &self.modifies {
            writer.indent()?;
            write!(writer.writer, "modifies ")?;
            writer.write(m)?;
            writeln!(writer.writer, ";")?;
        }
        Ok(())
    }
}

impl Writable for Type {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            Type::Bool => write!(writer.writer, "bool")?,
            Type::Bv(size) => write!(writer.writer, "bv{size}")?,
            Type::Int => write!(writer.writer, "int")?,
            Type::Map { key, value } => {
                write!(writer.writer, "[")?;
                writer.write(key.as_ref())?;
                write!(writer.writer, "]")?;
                writer.write(value.as_ref())?;
            }
        }
        Ok(())
    }
}

impl Writable for Literal {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            Literal::Bool(value) => {
                write!(writer.writer, "{}", value)?;
            }
            Literal::Bv { width, value } => {
                write!(writer.writer, "{value}bv{width}")?;
            }
            Literal::Int(value) => {
                write!(writer.writer, "{}", value)?;
            }
        }
        Ok(())
    }
}

impl Writable for UnaryOp {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            UnaryOp::Not => write!(writer.writer, "!")?,
            UnaryOp::Neg => write!(writer.writer, "-")?,
        }
        Ok(())
    }
}

impl Writable for BinaryOp {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            BinaryOp::Add => write!(writer.writer, "+")?,
            BinaryOp::Sub => write!(writer.writer, "-")?,
            BinaryOp::Mul => write!(writer.writer, "*")?,
            BinaryOp::Div => write!(writer.writer, "/")?,
            BinaryOp::Mod => write!(writer.writer, "%")?,
            BinaryOp::And => write!(writer.writer, "&&")?,
            BinaryOp::Or => write!(writer.writer, "||")?,
            BinaryOp::Eq => write!(writer.writer, "==")?,
            BinaryOp::Neq => write!(writer.writer, "!=")?,
            BinaryOp::Lt => write!(writer.writer, "<")?,
            BinaryOp::Gt => write!(writer.writer, ">")?,
            BinaryOp::Lte => write!(writer.writer, "<=")?,
            BinaryOp::Gte => write!(writer.writer, ">=")?,
        }
        Ok(())
    }
}
