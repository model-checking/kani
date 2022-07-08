// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::Expr;
use std::fmt::Debug;

/// Represents a contract on a function, loop, etc.
#[derive(Clone, Debug)]
pub enum Contract {
    FunctionContract { ensures: Spec, requires: Spec },
}

/// A `Spec` is a struct for representing the `requires`, `ensures`, and `assigns` clauses in a function contract.
/// A function contract can have multiple `__CPROVER_requires(...)` or `__CPROVER_ensures(...)` statements.
/// The expressions from all the statements are stored in a single vector `clauses`.
/// Furthermore, every expression is wrapped into a lambda expression on the CBMC side.
/// This is because CBMC generates a new symbol for each contract and the symbol needs to be self-contained.
/// That is, variables that may have only existed in the scope of a function declaration are
///     treated as binding variables in the lambda expression and are now available to the contract symbol.
/// A list of fresh symbols (one for each binding variable in the lambda expression) is stored in `temporary_symbols`.
/// The binding variables include the return value of the function (may be empty) and the list of function arguments.
#[derive(Clone, Debug)]
pub struct Spec {
    temporary_symbols: Vec<Expr>,
    clauses: Vec<Expr>,
}

/// Getters
impl Spec {
    pub fn temporary_symbols(&self) -> &Vec<Expr> {
        &self.temporary_symbols
    }

    pub fn clauses(&self) -> &Vec<Expr> {
        &self.clauses
    }
}

/// Constructor
impl Spec {
    pub fn new(temporary_symbols: Vec<Expr>, clauses: Vec<Expr>) -> Self {
        assert!(temporary_symbols.iter().all(|x| x.is_symbol()), "Variables must be symbols");
        Spec { temporary_symbols, clauses }
    }
}
