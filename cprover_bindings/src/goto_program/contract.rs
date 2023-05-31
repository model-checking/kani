// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{Expr, Location};
use std::fmt::Debug;

/// A `Contract` represents a code contract type.
/// A contract describes specifications (in the form of preconditions, postconditions, and invariants) of certain expressions.
/// Further details about the CBMC implementation can be found here -
/// <https://github.com/diffblue/cbmc/blob/develop/doc/cprover-manual/contracts.md>

/// Represents a contract on a function, loop, etc.
#[derive(Clone, Debug)]
pub enum ContractValue {
    FunctionContract { ensures: Vec<Spec>, requires: Vec<Spec>, assigns: Vec<Spec> },
}

/// The fields of `Contract` are kept private, and there are no getters that return mutable references.
/// This means that the only way to create and update `Contract`s is using the constructors and setters.
#[derive(Clone, Debug)]
pub struct Contract(ContractValue);

/// A `Spec` is a struct for representing the `requires`, `ensures`, and `assigns` clauses in a function contract.
/// Every expression inside a function contract clause is wrapped into a lambda expression on the CBMC side.
/// This is because CBMC generates a new symbol for each contract and the symbol needs to be self-contained.
/// That is, variables that may have only existed in the scope of a function declaration are
///     treated as binding variables in the lambda expression and hence made available to the contract symbol.
/// A list of fresh symbols (one for each binding variable in the lambda expression) is stored in `temporary_symbols`.
/// The binding variables include the return value of the function (may be empty) and the list of function arguments.
#[derive(Clone, Debug)]
pub struct Spec {
    temporary_symbols: Vec<Expr>,
    clause: Expr,
    location: Location,
}

/// Getters
impl Spec {
    pub fn temporary_symbols(&self) -> &Vec<Expr> {
        &self.temporary_symbols
    }

    pub fn clause(&self) -> &Expr {
        &self.clause
    }

    pub fn location(&self) -> &Location {
        &self.location
    }
}

impl Contract {
    pub fn value(&self) -> &ContractValue {
        &self.0
    }
}

/// Setters
impl Spec {
    pub fn with_location(mut self, loc: Location) -> Self {
        self.location = loc;
        self
    }
}

/// Constructor
impl Spec {
    pub fn new(temporary_symbols: Vec<Expr>, clause: Expr, location: Location) -> Self {
        assert!(temporary_symbols.iter().all(|x| x.is_symbol()), "Variables must be symbols");
        Spec { temporary_symbols, clause, location }
    }
}

impl Contract {
    pub fn function_contract(ensures: Vec<Spec>, requires: Vec<Spec>, assigns: Vec<Spec>) -> Self {
        assert!(
            assigns.iter().all(|x| x.clause().is_symbol()),
            "Assigns clause must contain symbols"
        );
        Contract(ContractValue::FunctionContract { ensures, requires, assigns })
    }
}
