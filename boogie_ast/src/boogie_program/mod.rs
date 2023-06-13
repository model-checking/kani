// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod writer;

struct TypeDeclaration {}
struct ConstDeclaration {}
struct VarDeclaration {}
struct Axiom {}

/// Boogie types
pub enum Type {
    /// Boolean
    Bool,

    /// Bit-vector of a given width, e.g. `bv32`
    Bv(usize),

    /// Unbounded integer
    Int,

    /// Map type, e.g. `[int]bool`
    Map { key: Box<Type>, value: Box<Type> },
}

/// Function and procedure parameters
pub struct Parameter {
    name: String,
    typ: Type,
}

/// Literal types
pub enum Literal {
    /// Boolean values: `true`/`false`
    Bool(bool),

    /// Bit-vector values, e.g. `5bv8`
    Bv {
        width: usize,
        value: String, // TODO: use bigint
    },

    /// Unbounded integer values, e.g. `1000`
    Int(String), // TODO: use bigint
}

/// Unary operators
pub enum UnaryOp {
    /// Logical negation
    Not,

    /// Arithmetic negative
    Neg,
}

pub enum BinaryOp {
    /// Logical AND
    And,

    /// Logical OR
    Or,

    /// Equality
    Eq,

    /// Inequality
    Neq,

    /// Less than
    Lt,

    /// Less than or equal
    Lte,

    /// Greater than
    Gt,

    /// Greater than or equal
    Gte,

    /// Addition
    Add,

    /// Subtraction
    Sub,

    /// Multiplication
    Mul,

    /// Division
    Div,

    /// Modulo
    Mod,
}

/// Expr types
pub enum Expr {
    /// Literal (constant)
    Literal(Literal),

    /// Variable
    Symbol { name: String },

    /// Unary operation
    UnaryOp { op: UnaryOp, operand: Box<Expr> },

    /// Binary operation
    BinaryOp { op: BinaryOp, left: Box<Expr>, right: Box<Expr> },
}

/// Statement types
pub enum Stmt {
    /// Assignment statement: `target := value;`
    Assignment { target: String, value: Expr },

    /// Assert statement: `assert condition;`
    Assert { condition: Expr },

    /// Assume statement: `assume condition;`
    Assume { condition: Expr },

    /// Statement block: `{ statements }`
    Block { statements: Vec<Stmt> },

    /// Break statement: `break;`
    /// A `break` in boogie can take a label, but this is probably not needed
    Break,

    /// Procedure call: `symbol(arguments);`
    Call { symbol: String, arguments: Vec<Expr> },

    /// Declaration statement: `var name: type;`
    Decl { name: String, typ: Type },

    /// If statement: `if (condition) { body } else { else_body }`
    If { condition: Expr, body: Box<Stmt>, else_body: Option<Box<Stmt>> },

    /// Goto statement: `goto label;`
    Goto { label: String },

    /// Label statement: `label:`
    Label { label: String },

    /// Return statement: `return;`
    Return,

    /// While statement: `while (condition) { body }`
    While { condition: Expr, body: Box<Stmt> },
}

/// Procedure specification
pub struct Contract {
    /// Pre-conditions
    requires: Vec<Expr>,
    /// Post-conditions
    ensures: Vec<Expr>,
    /// Modifies clauses
    // TODO: should be symbols and not expressions
    modifies: Vec<Expr>,
}

/// Procedure definition
pub struct Procedure {
    name: String,
    parameters: Vec<Parameter>,
    return_type: Vec<(String, Type)>,
    contract: Option<Contract>,
    body: Stmt,
}

impl Procedure {
    pub fn new(
        name: String,
        parameters: Vec<Parameter>,
        return_type: Vec<(String, Type)>,
        contract: Option<Contract>,
        body: Stmt,
    ) -> Self {
        Procedure { name, parameters, return_type, contract, body }
    }
}

/// Function definition
struct Function {}

/// A boogie program
pub struct BoogieProgram {
    type_declarations: Vec<TypeDeclaration>,
    const_declarations: Vec<ConstDeclaration>,
    var_declarations: Vec<VarDeclaration>,
    axioms: Vec<Axiom>,
    functions: Vec<Function>,
    procedures: Vec<Procedure>,
}

impl BoogieProgram {
    pub fn new() -> Self {
        BoogieProgram {
            type_declarations: Vec::new(),
            const_declarations: Vec::new(),
            var_declarations: Vec::new(),
            axioms: Vec::new(),
            functions: Vec::new(),
            procedures: Vec::new(),
        }
    }

    pub fn add_procedure(&mut self, procedure: Procedure) {
        self.procedures.push(procedure);
    }

    pub fn sample_program() -> Self {
        BoogieProgram {
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
                            value: Expr::Literal(Literal::Int("1".to_string())),
                        },
                        Stmt::Assignment {
                            target: "y".to_string(),
                            value: Expr::Literal(Literal::Int("2".to_string())),
                        },
                        Stmt::Assert {
                            condition: Expr::BinaryOp {
                                op: BinaryOp::Eq,
                                left: Box::new(Expr::Symbol { name: "x".to_string() }),
                                right: Box::new(Expr::Literal(Literal::Int("1".to_string()))),
                            },
                        },
                        Stmt::Assert {
                            condition: Expr::BinaryOp {
                                op: BinaryOp::Eq,
                                left: Box::new(Expr::Symbol { name: "y".to_string() }),
                                right: Box::new(Expr::Literal(Literal::Int("2".to_string()))),
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
        }
    }
}
