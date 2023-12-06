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
use num_bigint::Sign;

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

impl<'a, T: Write> Write for Writer<'a, T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl BoogieProgram {
    pub fn write_to<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        let mut writer = Writer::new(writer);

        if !self.type_declarations.is_empty() {
            writeln!(writer, "// Type declarations:")?;
            for _td in &self.type_declarations {
                todo!()
            }
        }
        if !self.const_declarations.is_empty() {
            writeln!(writer, "// Constant declarations:")?;
            for _const_decl in &self.const_declarations {
                todo!()
            }
        }
        if !self.var_declarations.is_empty() {
            writeln!(writer, "// Variable declarations:")?;
            for _var_decl in &self.var_declarations {
                todo!()
            }
        }
        if !self.axioms.is_empty() {
            writeln!(writer, "// Axioms:")?;
            for _a in &self.axioms {
                todo!()
            }
        }
        if !self.functions.is_empty() {
            writeln!(writer, "// Functions:")?;
            for f in &self.functions {
                f.write_to(&mut writer)?;
            }
        }
        if !self.procedures.is_empty() {
            writeln!(writer, "// Procedures:")?;
            for p in &self.procedures {
                p.write_to(&mut writer)?;
            }
        }
        Ok(())
    }
}

impl Function {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        // signature
        write!(writer, "function ")?;
        // attributes
        for attr in &self.attributes {
            write!(writer, "{{{attr}}} ")?;
        }
        write!(writer, "{}", self.name)?;
        // generics
        if !self.generics.is_empty() {
            write!(writer, "<")?;
            for (i, name) in self.generics.iter().enumerate() {
                if i > 0 {
                    write!(writer, ", ")?;
                }
                write!(writer, "{name}")?;
            }
            write!(writer, ">")?;
        }
        // parameters
        write!(writer, "(")?;
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(writer, ", ")?;
            }
            write!(writer, "{}: ", param.name)?;
            param.typ.write_to(writer)?;
        }
        write!(writer, ") returns (")?;
        self.return_type.write_to(writer)?;
        write!(writer, ")")?;
        if let Some(body) = &self.body {
            writeln!(writer, " {{")?;
            writer.increase_indent();
            writer.indent()?;
            body.write_to(writer)?;
            writer.decrease_indent();
            writer.newline()?;
            writeln!(writer, "}}")?;
        } else {
            writeln!(writer, ";")?;
        }
        writer.newline()?;
        Ok(())
    }
}

impl Procedure {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        // signature
        write!(writer, "procedure {}(", self.name)?;
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(writer, ",")?;
            }
            write!(writer, "{}: ", param.name)?;
            param.typ.write_to(writer)?;
        }
        write!(writer, ") ")?;
        if !self.return_type.is_empty() {
            write!(writer, "returns (")?;
            for (i, (name, typ)) in self.return_type.iter().enumerate() {
                if i > 0 {
                    write!(writer, ",")?;
                }
                write!(writer, "{name}: ")?;
                typ.write_to(writer)?;
            }
            write!(writer, ")")?;
        }
        writer.newline()?;

        // contract
        if let Some(contract) = &self.contract {
            writer.increase_indent();
            contract.write_to(writer)?;
            writer.decrease_indent();
        }
        writeln!(writer, "{{")?;
        writer.increase_indent();
        self.body.write_to(writer)?;
        writer.decrease_indent();
        writeln!(writer, "}}")?;
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
                write!(writer, "{name}")?;
            }
            Expr::UnaryOp { op, operand } => {
                op.write_to(writer)?;
                write!(writer, "(")?;
                operand.write_to(writer)?;
                write!(writer, ")")?;
            }
            Expr::BinaryOp { op, left, right } => {
                write!(writer, "(")?;
                left.write_to(writer)?;
                write!(writer, " ")?;
                op.write_to(writer)?;
                write!(writer, " ")?;
                right.write_to(writer)?;
                write!(writer, ")")?;
            }
            Expr::FunctionCall { symbol, arguments } => {
                write!(writer, "{symbol}(")?;
                for (i, a) in arguments.iter().enumerate() {
                    if i > 0 {
                        write!(writer, ", ")?;
                    }
                    a.write_to(writer)?;
                }
                write!(writer, ")")?;
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
                write!(writer, "{} := ", target)?;
                value.write_to(writer)?;
                writeln!(writer, ";")?;
            }
            Stmt::Assert { condition } => {
                writer.indent()?;
                write!(writer, "assert ")?;
                condition.write_to(writer)?;
                writeln!(writer, ";")?;
            }
            Stmt::Assume { condition } => {
                writer.indent()?;
                write!(writer, "assume ")?;
                condition.write_to(writer)?;
                writeln!(writer, ";")?;
            }
            Stmt::Block { statements } => {
                for s in statements {
                    s.write_to(writer)?;
                }
            }
            Stmt::Break => {
                writer.indent()?;
                writeln!(writer, "break;")?;
            }
            Stmt::Call { symbol, arguments } => {
                writer.indent()?;
                write!(writer, "{symbol}(")?;
                for (i, a) in arguments.iter().enumerate() {
                    if i > 0 {
                        write!(writer, ", ")?;
                    }
                    a.write_to(writer)?;
                }
                writeln!(writer, ");")?;
            }
            Stmt::Decl { name, typ } => {
                writer.indent()?;
                write!(writer, "var {}: ", name)?;
                typ.write_to(writer)?;
                writeln!(writer, ";")?;
            }
            Stmt::If { condition, body, else_body } => {
                writer.indent()?;
                write!(writer, "if (")?;
                condition.write_to(writer)?;
                writeln!(writer, ") {{")?;
                writer.increase_indent();
                body.write_to(writer)?;
                writer.decrease_indent();
                writer.indent()?;
                write!(writer, "}}")?;
                if let Some(else_body) = else_body {
                    writeln!(writer, " else {{")?;
                    writer.increase_indent();
                    else_body.write_to(writer)?;
                    writer.decrease_indent();
                    writer.indent()?;
                    write!(writer, "}}")?;
                }
                writeln!(writer)?;
            }
            Stmt::Goto { label } => {
                writer.indent()?;
                writeln!(writer, "goto {label};")?;
            }
            Stmt::Label { label } => {
                writer.indent()?;
                writeln!(writer, "{label}:")?;
            }
            Stmt::Return => {
                writer.indent()?;
                writeln!(writer, "return;")?;
            }
            Stmt::While { condition, body } => {
                writer.indent()?;
                write!(writer, "while (")?;
                condition.write_to(writer)?;
                writeln!(writer, ") {{")?;
                writer.increase_indent();
                body.write_to(writer)?;
                writer.decrease_indent();
                writeln!(writer, "}}")?;
            }
        }
        Ok(())
    }
}

impl Contract {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        for r in &self.requires {
            writer.indent()?;
            write!(writer, "requires ")?;
            r.write_to(writer)?;
            writeln!(writer, ";")?;
        }
        for e in &self.ensures {
            writer.indent()?;
            write!(writer, "ensures ")?;
            e.write_to(writer)?;
            writeln!(writer, ";")?;
        }
        for m in &self.modifies {
            writer.indent()?;
            write!(writer, "modifies ")?;
            m.write_to(writer)?;
            writeln!(writer, ";")?;
        }
        Ok(())
    }
}

impl Type {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            Type::Bool => write!(writer, "bool")?,
            Type::Bv(size) => write!(writer, "bv{size}")?,
            Type::Int => write!(writer, "int")?,
            Type::Map { key, value } => {
                write!(writer, "[")?;
                key.write_to(writer)?;
                write!(writer, "]")?;
                value.write_to(writer)?;
            }
            Type::Parameter { name } => write!(writer, "{name}")?,
        }
        Ok(())
    }
}

impl Literal {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            Literal::Bool(value) => {
                write!(writer, "{}", value)?;
            }
            Literal::Bv { width, value } => {
                if value.sign() != Sign::Minus {
                    write!(writer, "{value}bv{width}")?;
                } else {
                    todo!("Handle negative integers");
                }
            }
            Literal::Int(value) => {
                write!(writer, "{}", value)?;
            }
        }
        Ok(())
    }
}

impl UnaryOp {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            UnaryOp::Not => write!(writer, "!")?,
            UnaryOp::Neg => write!(writer, "-")?,
        }
        Ok(())
    }
}

impl BinaryOp {
    fn write_to<T: Write>(&self, writer: &mut Writer<T>) -> std::io::Result<()> {
        match self {
            BinaryOp::Add => write!(writer, "+")?,
            BinaryOp::Sub => write!(writer, "-")?,
            BinaryOp::Mul => write!(writer, "*")?,
            BinaryOp::Div => write!(writer, "/")?,
            BinaryOp::Mod => write!(writer, "%")?,
            BinaryOp::And => write!(writer, "&&")?,
            BinaryOp::Or => write!(writer, "||")?,
            BinaryOp::Eq => write!(writer, "==")?,
            BinaryOp::Neq => write!(writer, "!=")?,
            BinaryOp::Lt => write!(writer, "<")?,
            BinaryOp::Gt => write!(writer, ">")?,
            BinaryOp::Lte => write!(writer, "<=")?,
            BinaryOp::Gte => write!(writer, ">=")?,
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
            functions: vec![
                Function::new(
                    "isZero".into(),
                    Vec::new(),
                    vec![Parameter::new("x".into(), Type::Int)],
                    Type::Bool,
                    Some(Expr::BinaryOp {
                        op: BinaryOp::Eq,
                        left: Box::new(Expr::Symbol { name: "x".into() }),
                        right: Box::new(Expr::Literal(Literal::Int(0.into()))),
                    }),
                    vec![":inline".into()],
                ),
                Function::new(
                    "$BvAnd".into(),
                    vec!["T".into()],
                    vec![
                        Parameter::new("lhs".into(), Type::parameter("T".into())),
                        Parameter::new("rhs".into(), Type::parameter("T".into())),
                    ],
                    Type::parameter("T".into()),
                    None,
                    vec![":bvbuiltin \"bvand\"".into()],
                ),
            ],
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
// Functions:
function {:inline} isZero(x: int) returns (bool) {
  (x == 0)
}

function {:bvbuiltin \"bvand\"} $BvAnd<T>(lhs: T, rhs: T) returns (T);

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
