// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::ExprValue::*;
use super::{Expr, ExprValue, Stmt, StmtBody, SwitchCase, Symbol, SymbolTable, SymbolValues};
/// The confidence we have that the this goto expression correctly represents the desired semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    Low,
    Medium,
    High,
    Default,
}

pub struct ConfidenceVisitor<'a> {
    symbol_table: &'a SymbolTable,
    required_confidence: Confidence,
}

impl ConfidenceVisitor<'a> {
    pub fn new(
        symbol_table: &'a SymbolTable,
        required_confidence: Confidence,
    ) -> ConfidenceVisitor<'a> {
        Self { symbol_table, required_confidence }
    }
}
impl ConfidenceVisitor<'_> {
    pub fn visit_symbol_table(&mut self, st: &SymbolTable) -> SymbolTable {
        let mut new_st = SymbolTable::raw(st.machine_model().clone());
        for (_key, value) in st.iter() {
            new_st.insert(self.visit_symbol(value));
        }
        new_st
    }

    pub fn visit_symbol(&mut self, symbol: &Symbol) -> Symbol {
        let mut new_symbol = symbol.clone();
        match &symbol.value {
            SymbolValues::Expr(e) => {
                new_symbol.value = SymbolValues::Expr(self.visit_expr(e));
            }
            SymbolValues::Stmt(s) => new_symbol.value = SymbolValues::Stmt(self.visit_stmt(s)),
            SymbolValues::None => {}
        };
        new_symbol
    }

    pub fn visit_switchcase(&mut self, sc: &SwitchCase) -> SwitchCase {
        SwitchCase::new(self.visit_expr(sc.case()), self.visit_stmt(sc.body()))
    }

    pub fn visit_stmt(&mut self, stmt: &Stmt) -> Stmt {
        let loc = stmt.location().clone();
        match stmt.body() {
            StmtBody::Assign { lhs, rhs } => {
                Stmt::assign(self.visit_expr(lhs), self.visit_expr(rhs), loc)
            }
            StmtBody::Assume { cond } => Stmt::assume(self.visit_expr(cond), loc),
            StmtBody::AtomicBlock(body) => {
                let body = body.iter().map(|x| self.visit_stmt(x)).collect();
                Stmt::atomic_block(body).with_location(loc)
            }
            StmtBody::Block(body) => {
                let body = body.iter().map(|x| self.visit_stmt(x)).collect();
                Stmt::block(body).with_location(loc)
            }
            StmtBody::Break => stmt.clone(),
            StmtBody::Continue => stmt.clone(),
            StmtBody::Decl { lhs, value } => {
                let lhs = self.visit_expr(lhs);
                let value = value.as_ref().map(|x| self.visit_expr(x));
                Stmt::decl(lhs, value, loc)
            }
            StmtBody::Expression(e) => Stmt::code_expression(self.visit_expr(e), loc),
            StmtBody::For { init, cond, update, body } => {
                let init = self.visit_stmt(init);
                let cond = self.visit_expr(cond);
                let update = self.visit_stmt(update);
                let body = self.visit_stmt(body);
                Stmt::for_loop(init, cond, update, body, loc)
            }
            StmtBody::FunctionCall { lhs, function, arguments } => {
                let lhs = lhs.as_ref().map(|x| self.visit_expr(x));
                let function = self.visit_expr(function);
                let arguments = arguments.iter().map(|x| self.visit_expr(x)).collect();
                Stmt::function_call(lhs, function, arguments, loc)
            }
            StmtBody::Goto(_) => stmt.clone(),
            StmtBody::Ifthenelse { i, t, e } => {
                let i = self.visit_expr(i);
                let t = self.visit_stmt(t);
                let e = e.as_ref().map(|x| self.visit_stmt(x));
                Stmt::if_then_else(i, t, e, loc)
            }
            StmtBody::Label { label, body } => {
                let body = self.visit_stmt(body);
                body.with_label(label.to_string()).with_location(loc)
            }
            StmtBody::Return(e) => {
                let e = e.as_ref().map(|x| self.visit_expr(x));
                Stmt::ret(e, loc)
            }
            StmtBody::Skip => stmt.clone(),
            StmtBody::Switch { control, cases, default } => {
                let control = self.visit_expr(control);
                let cases = cases.iter().map(|x| self.visit_switchcase(x)).collect();
                let default = default.as_ref().map(|x| self.visit_stmt(x));
                Stmt::switch(control, cases, default, loc)
            }
            StmtBody::While { cond, body } => {
                let cond = self.visit_expr(cond);
                let body = self.visit_stmt(body);
                Stmt::while_loop(cond, body, loc)
            }
        }
    }

    pub fn visit_expr(&mut self, expr: &Expr) -> Expr {
        let updated = match expr.value() {
            AddressOf(e) => {
                let e = self.visit_expr(e);
                e.address_of()
            }
            Array { elems } => {
                let elems = elems.iter().map(|x| self.visit_expr(x)).collect();
                Expr::array_expr(expr.typ().clone(), elems)
            }
            ArrayOf { elem } => {
                let elem = self.visit_expr(elem);
                let width = elem.typ().width().unwrap();
                elem.array_constant(width)
            }
            Assign { left, right } => {
                let left = self.visit_expr(left);
                let right = self.visit_expr(right);
                left.assign_expr(right)
            }
            BinOp { op, lhs, rhs } => {
                let lhs = self.visit_expr(lhs);
                let rhs = self.visit_expr(rhs);
                lhs.binop(*op, rhs)
            }
            BoolConstant(_) => expr.clone(),
            ByteExtract { e, offset } => {
                assert!(*offset == 0);
                let e = self.visit_expr(e);
                e.transmute_to(expr.typ().clone(), self.symbol_table)
            }
            CBoolConstant(_) => expr.clone(),
            Dereference(e) => {
                let e = self.visit_expr(e);
                e.dereference()
            }
            DoubleConstant(_) => expr.clone(),
            FloatConstant(_) => expr.clone(),
            FunctionCall { function, arguments } => {
                let function = self.visit_expr(function);
                let arguments = arguments.iter().map(|x| self.visit_expr(x)).collect();
                function.call(arguments)
            }
            If { c, t, e } => {
                let c = self.visit_expr(c);
                let t = self.visit_expr(t);
                let e = self.visit_expr(e);
                c.ternary(t, e)
            }
            Index { array, index } => {
                let array = self.visit_expr(array);
                let index = self.visit_expr(index);
                array.index(index)
            }
            IntConstant(_) => expr.clone(),
            Member { lhs, field } => {
                let lhs = self.visit_expr(lhs);
                lhs.member(field, self.symbol_table)
            }
            Nondet => expr.clone(),
            PointerConstant(_) => expr.clone(),
            SelfOp { op, e } => {
                let e = self.visit_expr(e);
                e.self_op(*op)
            }
            StatementExpression { statements } => {
                let statements = statements.iter().map(|x| self.visit_stmt(x)).collect();
                Expr::statement_expression(statements, expr.typ().clone())
            }
            StringConstant { .. } => expr.clone(),
            Struct { values } => {
                let fields = self.symbol_table.lookup_fields_in_type(expr.typ()).unwrap();
                let values = values.iter().map(|x| self.visit_expr(x)).collect();
                Expr::struct_expr_with_explicit_padding(expr.typ().clone(), fields, values)
            }
            ExprValue::Symbol { .. } => expr.clone(),
            Typecast(e) => {
                let e = self.visit_expr(e);
                e.cast_to(expr.typ().clone())
            }
            Union { value, field } => {
                todo!()
            }
            UnOp { op, e } => {
                let e = self.visit_expr(e);
                e.unop(*op)
            }
        }
        .with_location(expr.location().clone())
        .with_confidence(expr.confidence());
        self.assert_confidence(updated)
    }

    pub fn assert_confidence(&mut self, expr: Expr) -> Expr {
        if expr.confidence() < self.required_confidence {
            let typ = expr.typ().clone();
            let assert = Stmt::assert_false(
                &format!(
                    "Confidence {:?} is below required {:?}",
                    expr.confidence(),
                    self.required_confidence
                ),
                expr.location().clone(),
            );
            Expr::statement_expression(vec![assert, expr.as_stmt()], typ)
        } else {
            expr
        }
    }
}
