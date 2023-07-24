// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A writer for Boogie programs.
//! Generates a text Boogie program with the following format:
//! ```ignore
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
//!   requires <pre-condition1>;
//!   requires <pre-condition2>;
//!   ...
//!   ensures <post-condition1>;
//!   ensures <post-condition2>;
//!   ...
//!   modifies <var1>, <var2>, ...;
//! {
//!   <body>
//! }
//! ...
//!
//! ```
use crate::boogie_program::*;

use std::io::Write;

/// A writer for Boogie programs.
struct Writer<'a, T: Write> {
    writer: &'a mut T,
    indentation: usize,
}

impl<'a, T: Write> Writer<'a, T> {
    fn new(writer: &'a mut T) -> Self {
        Self { writer, indentation: 0 }
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
                p.write_to(&mut writer)?;
            }
        }
        Ok(())
    }
}

impl Procedure {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        // signature
        write!(writer.writer, "procedure {}(", self.name)?;
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(writer.writer, ",")?;
            }
            write!(writer.writer, "{}: ", param.name)?;
            param.typ.write_to(writer)?;
        }
        write!(writer.writer, ") ")?;
        if !self.return_type.is_empty() {
            write!(writer.writer, "returns (")?;
            for (i, (name, typ)) in self.return_type.iter().enumerate() {
                if i > 0 {
                    write!(writer.writer, ",")?;
                }
                write!(writer.writer, "{name}: ")?;
                typ.write_to(writer)?;
            }
            write!(writer.writer, ")")?;
        }
        writer.newline()?;

        // contract
        if let Some(contract) = &self.contract {
            writer.increase_indent();
            contract.write_to(writer)?;
            writer.decrease_indent();
        }
        writeln!(writer.writer, "{{")?;
        writer.increase_indent();
        self.body.write_to(writer)?;
        writer.decrease_indent();
        writeln!(writer.writer, "}}")?;
        Ok(())
    }
}

impl Expr {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            Expr::Literal(value) => {
                value.write_to(writer)?;
            }
            Expr::Symbol { name } => {
                write!(writer.writer, "{name}")?;
            }
            Expr::UnaryOp { op, operand } => {
                op.write_to(writer)?;
                write!(writer.writer, "(")?;
                operand.write_to(writer)?;
                write!(writer.writer, ")")?;
            }
            Expr::BinaryOp { op, left, right } => {
                write!(writer.writer, "(")?;
                left.write_to(writer)?;
                write!(writer.writer, " ")?;
                op.write_to(writer)?;
                write!(writer.writer, " ")?;
                right.write_to(writer)?;
                write!(writer.writer, ")")?;
            }
        }
        Ok(())
    }
}

impl Stmt {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            Stmt::Assignment { target, value } => {
                writer.indent()?;
                write!(writer.writer, "{} := ", target)?;
                value.write_to(writer)?;
                writeln!(writer.writer, ";")?;
            }
            Stmt::Assert { condition } => {
                writer.indent()?;
                write!(writer.writer, "assert ")?;
                condition.write_to(writer)?;
                writeln!(writer.writer, ";")?;
            }
            Stmt::Assume { condition } => {
                writer.indent()?;
                write!(writer.writer, "assume ")?;
                condition.write_to(writer)?;
                writeln!(writer.writer, ";")?;
            }
            Stmt::Block { statements } => {
                for s in statements {
                    s.write_to(writer)?;
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
                    a.write_to(writer)?;
                }
                writeln!(writer.writer, ");")?;
            }
            Stmt::Decl { name, typ } => {
                writer.indent()?;
                write!(writer.writer, "var {}: ", name)?;
                typ.write_to(writer)?;
                writeln!(writer.writer, ";")?;
            }
            Stmt::If { condition, body, else_body } => {
                writer.indent()?;
                write!(writer.writer, "if (")?;
                condition.write_to(writer)?;
                writeln!(writer.writer, ") {{")?;
                writer.increase_indent();
                body.write_to(writer)?;
                writer.decrease_indent();
                writer.indent()?;
                write!(writer.writer, "}}")?;
                if let Some(else_body) = else_body {
                    writeln!(writer.writer, " else {{")?;
                    writer.increase_indent();
                    else_body.write_to(writer)?;
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
                condition.write_to(writer)?;
                writeln!(writer.writer, ") {{")?;
                writer.increase_indent();
                body.write_to(writer)?;
                writer.decrease_indent();
                writeln!(writer.writer, "}}")?;
            }
        }
        Ok(())
    }
}

impl Contract {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        for r in &self.requires {
            writer.indent()?;
            write!(writer.writer, "requires ")?;
            r.write_to(writer)?;
            writeln!(writer.writer, ";")?;
        }
        for e in &self.ensures {
            writer.indent()?;
            write!(writer.writer, "ensures ")?;
            e.write_to(writer)?;
            writeln!(writer.writer, ";")?;
        }
        for m in &self.modifies {
            writer.indent()?;
            write!(writer.writer, "modifies ")?;
            m.write_to(writer)?;
            writeln!(writer.writer, ";")?;
        }
        Ok(())
    }
}

impl Type {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            Type::Bool => write!(writer.writer, "bool")?,
            Type::Bv(size) => write!(writer.writer, "bv{size}")?,
            Type::Int => write!(writer.writer, "int")?,
            Type::Map { key, value } => {
                write!(writer.writer, "[")?;
                key.write_to(writer)?;
                write!(writer.writer, "]")?;
                value.write_to(writer)?;
            }
        }
        Ok(())
    }
}

impl Literal {
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

impl UnaryOp {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            UnaryOp::Not => write!(writer.writer, "!")?,
            UnaryOp::Neg => write!(writer.writer, "-")?,
        }
        Ok(())
    }
}

impl BinaryOp {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_program() {
        let program = BoogieProgram {
            type_declarations: Vec::new(),
            const_declarations: Vec::new(),
            var_declarations: Vec::new(),
            axioms: Vec::new(),
            functions: Vec::new(),
            procedures: vec![Procedure {
                name: "main".to_string(),
                parameters: Vec::new(),
                return_type: vec![("z".to_string(), Type::Bool)],
                contract: Some(Contract {
                    requires: Vec::new(),
                    ensures: vec![Expr::BinaryOp {
                        op: BinaryOp::Eq,
                        left: Box::new(Expr::Symbol { name: "z".to_string() }),
                        right: Box::new(Expr::Literal(Literal::Bool(true))),
                    }],
                    modifies: Vec::new(),
                }),
                body: Stmt::Block {
                    statements: vec![
                        Stmt::Decl { name: "x".to_string(), typ: Type::Int },
                        Stmt::Decl { name: "y".to_string(), typ: Type::Int },
                        Stmt::Assignment {
                            target: "x".to_string(),
                            value: Expr::Literal(Literal::Int(1.into())),
                        },
                        Stmt::Assignment {
                            target: "y".to_string(),
                            value: Expr::Literal(Literal::Int(2.into())),
                        },
                        Stmt::Assert {
                            condition: Expr::BinaryOp {
                                op: BinaryOp::Eq,
                                left: Box::new(Expr::Symbol { name: "x".to_string() }),
                                right: Box::new(Expr::Literal(Literal::Int(1.into()))),
                            },
                        },
                        Stmt::Assert {
                            condition: Expr::BinaryOp {
                                op: BinaryOp::Eq,
                                left: Box::new(Expr::Symbol { name: "y".to_string() }),
                                right: Box::new(Expr::Literal(Literal::Int(2.into()))),
                            },
                        },
                        Stmt::If {
                            condition: Expr::BinaryOp {
                                op: BinaryOp::Lt,
                                left: Box::new(Expr::Symbol { name: "x".to_string() }),
                                right: Box::new(Expr::Symbol { name: "y".to_string() }),
                            },
                            body: Box::new(Stmt::Assignment {
                                target: "z".to_string(),
                                value: Expr::Literal(Literal::Bool(true)),
                            }),
                            else_body: Some(Box::new(Stmt::Assignment {
                                target: "z".to_string(),
                                value: Expr::Literal(Literal::Bool(false)),
                            })),
                        },
                    ],
                },
            }],
        };

        let mut v = Vec::new();
        program.write_to(&mut v).unwrap();
        let program_text = String::from_utf8(v).unwrap().to_string();

        let expected = String::from(
            "\
// Procedures:
procedure main() returns (z: bool)
  ensures (z == true);
{
  var x: int;
  var y: int;
  x := 1;
  y := 2;
  assert (x == 1);
  assert (y == 2);
  if ((x < y)) {
    z := true;
  } else {
    z := false;
  }
}
",
        );
        assert_eq!(program_text, expected);
    }
}
