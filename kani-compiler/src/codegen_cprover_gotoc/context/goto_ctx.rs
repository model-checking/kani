// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Kani can be thought of as a translator from an MIR context to a goto context.
//! This struct `GotocCtx<'tcx>` defined in this file, tracks both views of information.
//! In particular
//!   - `tcx` of the struct represents the MIR view
//!   - `symbol_table` represents the collected intermediate codegen results
//!   - the remaining fields represent temporary metadata held to assist in codegen.
//!
//! This file is for defining the data-structure itself.
//!   1. Defines `GotocCtx<'tcx>`
//!   2. Provides constructors, getters and setters for the context.
//!
//! Any MIR specific functionality (e.g. codegen etc) should live in specialized files that use
//! this structure as input.
use super::current_fn::CurrentFnCtx;
use super::vtable_ctx::VtableCtx;
use crate::codegen_cprover_gotoc::UnsupportedConstructs;
use crate::codegen_cprover_gotoc::overrides::{GotocHooks, fn_hooks};
use crate::codegen_cprover_gotoc::utils::full_crate_name;
use crate::kani_middle::transform::BodyTransformation;
use crate::kani_queries::QueryDb;
use cbmc::goto_program::{
    CIntType, DatatypeComponent, Expr, ExprValue, Location, Stmt, StmtBody, SwitchCase, Symbol,
    SymbolTable, SymbolValues, Type,
};
use cbmc::utils::aggr_tag;
use cbmc::{InternedString, MachineModel};
use rustc_abi::{HasDataLayout, TargetDataLayout};
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::span_bug;
use rustc_middle::ty::layout::{
    FnAbiError, FnAbiOfHelpers, FnAbiRequest, HasTyCtxt, HasTypingEnv, LayoutError,
    LayoutOfHelpers, TyAndLayout,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;
use rustc_span::source_map::respan;
use rustc_target::callconv::FnAbi;
use stable_mir::mir::Body;
use stable_mir::mir::mono::Instance;
use stable_mir::ty::Allocation;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Debug;

pub struct GotocCtx<'tcx> {
    /// the typing context
    pub tcx: TyCtxt<'tcx>,
    /// a snapshot of the query values. The queries shouldn't change at this point,
    /// so we just keep a copy.
    pub queries: QueryDb,
    /// the generated symbol table for gotoc
    pub symbol_table: SymbolTable,
    pub hooks: GotocHooks,
    /// the full crate name, including versioning info
    pub full_crate_name: String,
    /// a global counter for generating unique names for global variables
    pub global_var_count: u64,
    /// map a global allocation to a name in the symbol table
    pub alloc_map: FxHashMap<Allocation, String>,
    /// map (trait, method) pairs to possible implementations
    pub vtable_ctx: VtableCtx,
    pub current_fn: Option<CurrentFnCtx<'tcx>>,
    pub type_map: FxHashMap<InternedString, Ty<'tcx>>,
    /// map from symbol identifier to string literal
    /// TODO: consider making the map from Expr to String instead
    pub str_literals: FxHashMap<InternedString, String>,
    /// a global counter for generating unique IDs for checks
    pub global_checks_count: u64,
    /// A map of unsupported constructs that were found while codegen
    pub unsupported_constructs: UnsupportedConstructs,
    /// A map of concurrency constructs that are treated sequentially.
    /// We collect them and print one warning at the end if not empty instead of printing one
    /// warning at each occurrence.
    pub concurrent_constructs: UnsupportedConstructs,
    /// The body transformation agent.
    pub transformer: BodyTransformation,
    /// If there exist some usage of loop contracts int context.
    pub has_loop_contracts: bool,
}

/// Constructor
impl<'tcx> GotocCtx<'tcx> {
    pub fn new(
        tcx: TyCtxt<'tcx>,
        queries: QueryDb,
        machine_model: &MachineModel,
        transformer: BodyTransformation,
    ) -> GotocCtx<'tcx> {
        let fhks = fn_hooks();
        let symbol_table = SymbolTable::new(machine_model.clone());
        let emit_vtable_restrictions = queries.args().emit_vtable_restrictions;
        GotocCtx {
            tcx,
            queries,
            symbol_table,
            hooks: fhks,
            full_crate_name: full_crate_name(tcx),
            global_var_count: 0,
            alloc_map: FxHashMap::default(),
            vtable_ctx: VtableCtx::new(emit_vtable_restrictions),
            current_fn: None,
            type_map: FxHashMap::default(),
            str_literals: FxHashMap::default(),
            global_checks_count: 0,
            unsupported_constructs: FxHashMap::default(),
            concurrent_constructs: FxHashMap::default(),
            transformer,
            has_loop_contracts: false,
        }
    }
}

/// Getters
impl<'tcx> GotocCtx<'tcx> {
    pub fn current_fn(&self) -> &CurrentFnCtx<'tcx> {
        self.current_fn.as_ref().unwrap()
    }

    pub fn current_fn_mut(&mut self) -> &mut CurrentFnCtx<'tcx> {
        self.current_fn.as_mut().unwrap()
    }
}

/// Generate variables
impl GotocCtx<'_> {
    /// Declare a local variable.
    /// Handles the bookkeeping of:
    /// - creating the symbol
    /// - inserting it into the symbol table
    /// - adding the declaration to the local function
    pub fn declare_variable<T: Into<InternedString>, U: Into<InternedString>>(
        &mut self,
        name: T,
        base_name: U,
        t: Type,
        value: Option<Expr>,
        l: Location,
    ) -> Symbol {
        let sym = Symbol::variable(name, base_name, t, l);
        self.symbol_table.insert(sym.clone());
        self.current_fn_mut().push_onto_block(Stmt::decl(sym.to_expr(), value, l));
        sym
    }

    /// Given a counter `c` a function name `fname, and a prefix `prefix`, generates a new function local variable
    /// It is an error to reuse an existing `c`, `fname` `prefix` tuple.
    fn gen_stack_variable(
        &mut self,
        c: u64,
        fname: &str,
        prefix: &str,
        t: Type,
        loc: Location,
    ) -> Symbol {
        let base_name = format!("{prefix}_{c}");
        let name = format!("{fname}::1::{base_name}");
        let symbol = Symbol::variable(name, base_name, t, loc);
        self.symbol_table.insert(symbol.clone());
        symbol
    }

    /// Generate a new function local variable that can be used as a temporary
    /// in Kani expressions and declare it with the specified (optional) value
    pub fn decl_temp_variable(
        &mut self,
        t: Type,
        value: Option<Expr>,
        loc: Location,
    ) -> (Expr, Stmt) {
        let c = self.current_fn_mut().get_and_incr_counter();
        let var = self.gen_stack_variable(c, &self.current_fn().name(), "temp", t, loc).to_expr();
        let value = value.or_else(|| self.codegen_default_initializer(&var));
        let decl = Stmt::decl(var.clone(), value, loc);
        (var, decl)
    }
}

/// Symbol table related
impl<'tcx> GotocCtx<'tcx> {
    /// Ensures that the `name` appears in the Symbol table.
    /// If it doesn't, inserts it using `f`.
    pub fn ensure<
        F: FnOnce(&mut GotocCtx<'tcx>, InternedString) -> Symbol,
        T: Into<InternedString>,
    >(
        &mut self,
        name: T,
        f: F,
    ) -> &Symbol {
        let name = name.into();
        if !self.symbol_table.contains(name) {
            let sym = f(self, name);
            self.symbol_table.insert(sym);
        }
        self.symbol_table.lookup(name).unwrap()
    }

    /// Ensures that a global variable `name` appears in the Symbol table and is initialized.
    ///
    /// This will add the symbol to the Symbol Table if not inserted yet.
    /// This will register the initialization function if not initialized yet.
    ///   - This case can happen for static variables, since they are declared first.
    pub fn ensure_global_var_init<T, F>(
        &mut self,
        name: T,
        is_file_local: bool,
        is_const: bool,
        t: Type,
        loc: Location,
        init: F,
    ) -> &mut Symbol
    where
        T: Into<InternedString> + Clone + Debug,
        F: Fn(&mut GotocCtx, Symbol) -> Expr,
    {
        let sym = self.ensure_global_var(name.clone(), is_file_local, t, loc);
        sym.set_is_static_const(is_const);
        if matches!(sym.value, SymbolValues::None) {
            // Clone sym so we can use `&mut self`.
            let sym = sym.clone();
            let init_expr = SymbolValues::Expr(init(self, sym));
            // Need to lookup again since symbol table might've changed.
            let sym = self.symbol_table.lookup_mut(name).unwrap();
            sym.value = init_expr;
            sym
        } else {
            self.symbol_table.lookup_mut(name).unwrap()
        }
    }

    /// Ensures that a global variable `name` appears in the Symbol table.
    ///
    /// This will add the symbol to the Symbol Table if not inserted yet.
    pub fn ensure_global_var<T: Into<InternedString> + Clone>(
        &mut self,
        name: T,
        is_file_local: bool,
        t: Type,
        loc: Location,
    ) -> &mut Symbol {
        let sym_name = name.clone().into();
        if !self.symbol_table.contains(sym_name) {
            tracing::debug!(?sym_name, "ensure_global_var insert");
            let sym = Symbol::static_variable(sym_name, sym_name, t, loc)
                .with_is_file_local(is_file_local)
                .with_is_hidden(false);
            self.symbol_table.insert(sym.clone());
        }
        self.symbol_table.lookup_mut(sym_name).unwrap()
    }

    /// Ensures that a struct with name `struct_name` appears in the symbol table.
    /// If it doesn't, inserts it using `f`.
    /// Returns: a struct-tag referencing the inserted struct.
    pub fn ensure_struct<
        T: Into<InternedString>,
        U: Into<InternedString>,
        F: FnOnce(&mut GotocCtx<'tcx>, InternedString) -> Vec<DatatypeComponent>,
    >(
        &mut self,
        struct_name: T,
        pretty_name: U,
        f: F,
    ) -> Type {
        let struct_name = struct_name.into();

        assert!(!struct_name.starts_with("tag-"));
        if !self.symbol_table.contains(aggr_tag(struct_name)) {
            let pretty_name = pretty_name.into();
            // Prevent recursion by inserting an incomplete value.
            self.symbol_table.insert(Symbol::incomplete_struct(struct_name, pretty_name));
            let components = f(self, struct_name);
            let struct_name: InternedString = struct_name;
            let sym = Symbol::struct_type(struct_name, pretty_name, components);
            self.symbol_table.replace_with_completion(sym);
        }
        Type::struct_tag(struct_name)
    }

    /// Ensures that a union with name `union_name` appears in the symbol table.
    /// If it doesn't, inserts it using `f`.
    /// Returns: a union-tag referencing the inserted struct.
    pub fn ensure_union<
        T: Into<InternedString>,
        U: Into<InternedString>,
        F: FnOnce(&mut GotocCtx<'tcx>, InternedString) -> Vec<DatatypeComponent>,
    >(
        &mut self,
        union_name: T,
        pretty_name: U,
        f: F,
    ) -> Type {
        let union_name = union_name.into();
        let pretty_name = pretty_name.into();
        assert!(!union_name.starts_with("tag-"));
        if !self.symbol_table.contains(aggr_tag(union_name)) {
            // Prevent recursion by inserting an incomplete value.
            self.symbol_table.insert(Symbol::incomplete_union(union_name, pretty_name));
            let components = f(self, union_name);
            let sym = Symbol::union_type(union_name, pretty_name, components);
            self.symbol_table.replace_with_completion(sym);
        }
        Type::union_tag(union_name)
    }
}

/// Quantifiers Related
impl GotocCtx<'_> {
    /// Find all quantifier expressions and recursively inline functions in the quantifier bodies.
    pub fn handle_quantifiers(&mut self) {
        // Store the found quantifiers and the inlined results.
        let mut to_modify: BTreeMap<InternedString, SymbolValues> = BTreeMap::new();
        for (key, symbol) in self.symbol_table.iter() {
            if let SymbolValues::Stmt(stmt) = &symbol.value {
                let new_stmt_val = SymbolValues::Stmt(self.handle_quantifiers_in_stmt(stmt));
                to_modify.insert(*key, new_stmt_val);
            }
        }

        // Update the found quantifiers with the inlined results.
        for (key, symbol_value) in to_modify {
            self.symbol_table.lookup_mut(key).unwrap().update(symbol_value);
        }
    }

    /// Find all quantifier expressions in `stmt` and recursively inline functions.
    fn handle_quantifiers_in_stmt(&self, stmt: &Stmt) -> Stmt {
        match &stmt.body() {
            // According to the hook handling for quantifiers, quantifier expressions must be of form
            // lhs = typecast(qex, c_bool)
            // where qex is either a forall-expression or an exists-expression.
            StmtBody::Assign { lhs, rhs } => {
                let new_rhs = match &rhs.value() {
                    ExprValue::Typecast(quantified_expr) => match &quantified_expr.value() {
                        ExprValue::Forall { variable, domain } => {
                            // We store the function symbols we have inlined to avoid recursion.
                            let mut visited_func_symbols: HashSet<InternedString> = HashSet::new();
                            // We count the number of function that we have inlined, and use the count to
                            // make inlined labeled unique.
                            let mut suffix_count: u16 = 0;

                            let end_stmt = Stmt::code_expression(
                                self.inline_function_calls_in_expr(
                                    domain,
                                    &mut visited_func_symbols,
                                    &mut suffix_count,
                                )
                                .unwrap(),
                                *domain.location(),
                            );

                            // Make the result a statement expression.
                            let res = Expr::forall_expr(
                                Type::Bool,
                                variable.clone(),
                                Expr::statement_expression(
                                    vec![Stmt::skip(*domain.location()), end_stmt],
                                    Type::Bool,
                                    *domain.location(),
                                ),
                            );
                            res.cast_to(Type::CInteger(CIntType::Bool))
                        }
                        ExprValue::Exists { variable, domain } => {
                            // We store the function symbols we have inlined to avoid recursion.
                            let mut visited_func_symbols: HashSet<InternedString> = HashSet::new();
                            // We count the number of function that we have inlined, and use the count to
                            // make inlined labeled unique.
                            let mut suffix_count = 0;

                            let end_stmt = Stmt::code_expression(
                                self.inline_function_calls_in_expr(
                                    domain,
                                    &mut visited_func_symbols,
                                    &mut suffix_count,
                                )
                                .unwrap(),
                                *domain.location(),
                            );

                            // Make the result a statement expression.
                            let res = Expr::exists_expr(
                                Type::Bool,
                                variable.clone(),
                                Expr::statement_expression(
                                    vec![Stmt::skip(*domain.location()), end_stmt],
                                    Type::Bool,
                                    *domain.location(),
                                ),
                            );
                            res.cast_to(Type::CInteger(CIntType::Bool))
                        }
                        _ => rhs.clone(),
                    },
                    _ => rhs.clone(),
                };
                Stmt::assign(lhs.clone(), new_rhs, *stmt.location())
            }
            // Recursively find quantifier expressions.
            StmtBody::Block(stmts) => Stmt::block(
                stmts.iter().map(|stmt| self.handle_quantifiers_in_stmt(stmt)).collect(),
                *stmt.location(),
            ),
            StmtBody::Label { label, body } => {
                self.handle_quantifiers_in_stmt(body).with_label(*label)
            }
            _ => stmt.clone(),
        }
    }

    /// Count and return the number of return statements in `stmt`.
    fn count_return_stmts(stmt: &Stmt) -> usize {
        match stmt.body() {
            StmtBody::Return(_) => 1,
            StmtBody::Block(stmts) => stmts.iter().map(Self::count_return_stmts).sum(),
            StmtBody::Label { label: _, body } => Self::count_return_stmts(body),
            _ => 0,
        }
    }

    /// Rewrite return statements in `stmt` with a goto statement to `end_label`.
    /// It also stores the return symbol in `return_symbol`.
    fn rewrite_return_stmt_with_goto(
        stmt: &Stmt,
        return_symbol: &mut Option<Expr>,
        end_label: &InternedString,
    ) -> Stmt {
        match stmt.body() {
            StmtBody::Return(Some(expr)) => {
                if let ExprValue::Symbol { ref identifier } = expr.value() {
                    *return_symbol = Some(Expr::symbol_expression(*identifier, expr.typ().clone()));
                    Stmt::goto(*end_label, *stmt.location())
                } else {
                    panic!("Expected symbol expression in return statement");
                }
            }
            StmtBody::Block(stmts) => Stmt::block(
                stmts
                    .iter()
                    .map(|s| Self::rewrite_return_stmt_with_goto(s, return_symbol, end_label))
                    .collect(),
                *stmt.location(),
            ),
            StmtBody::Label { label, body } => {
                Self::rewrite_return_stmt_with_goto(body, return_symbol, end_label)
                    .with_label(*label)
            }
            _ => stmt.clone(),
        }
    }

    /// Append a given suffix to all labels and goto destinations in `stmt`.
    fn append_suffix_to_stmt(stmt: &Stmt, suffix: &str) -> Stmt {
        match stmt.body() {
            StmtBody::Label { label, body } => {
                let new_label = format!("{}{}", label, suffix);
                Self::append_suffix_to_stmt(body, suffix).with_label(new_label)
            }
            StmtBody::Goto { dest, .. } => {
                let new_target = format!("{}{}", dest, suffix);
                Stmt::goto(new_target, *stmt.location())
            }
            StmtBody::Block(stmts) => Stmt::block(
                stmts.iter().map(|s| Self::append_suffix_to_stmt(s, suffix)).collect(),
                *stmt.location(),
            ),
            StmtBody::Ifthenelse { i, t, e } => Stmt::if_then_else(
                i.clone(),
                Self::append_suffix_to_stmt(t, suffix),
                e.clone().map(|s| Self::append_suffix_to_stmt(&s, suffix)),
                *stmt.location(),
            ),
            StmtBody::Switch { control, cases, default } => {
                // Append the suffix to each case
                let new_cases: Vec<_> = cases
                    .iter()
                    .map(|case| {
                        let new_body = Self::append_suffix_to_stmt(case.body(), suffix);
                        SwitchCase::new(case.case().clone(), new_body)
                    })
                    .collect();

                // Append the suffix to the default case, if it exists
                let new_default =
                    default.as_ref().map(|stmt| Self::append_suffix_to_stmt(stmt, suffix));

                // Construct the new switch statement
                Stmt::switch(control.clone(), new_cases, new_default, *stmt.location())
            }
            StmtBody::While { .. } | StmtBody::For { .. } => {
                unimplemented!()
            }
            _ => stmt.clone(),
        }
    }

    /// Recursively inline all function calls in `expr`.
    /// `visited_func_symbols` contain all function symbols in the stack.
    /// `suffix_count` is used to make inlined labels unique.
    fn inline_function_calls_in_expr(
        &self,
        expr: &Expr,
        visited_func_symbols: &mut HashSet<InternedString>,
        suffix_count: &mut u16,
    ) -> Option<Expr> {
        match &expr.value() {
            // For function call expression, we find the function symbol and function body from the
            // symbol table for inlining.
            ExprValue::FunctionCall { function, arguments } => {
                if let ExprValue::Symbol { identifier } = &function.value() {
                    // Check if the function symbol exists in the symbol table
                    if let Some(function_body) =
                        self.symbol_table.lookup(*identifier).and_then(|sym| match &sym.value {
                            SymbolValues::Stmt(stmt) => Some(stmt),
                            _ => None,
                        })
                    {
                        // For function calls to foo(args) where the definition of foo is
                        // fn foo(params) {
                        //      body;
                        //      return res;
                        // }
                        // The inlining result will be a statement expression
                        // {
                        //  DECL    params
                        //  ASSIGN  params = args
                        //  inline(body)
                        //  GOTO    end_label
                        //  end_label:
                        //  EXPRESSION  res
                        // }
                        // where res is the end expression of the statement expression.

                        // Keep suffix unique in difference inlining.
                        *suffix_count += 1;

                        // Use call stacks to avoid recursion.
                        assert!(
                            !visited_func_symbols.contains(identifier),
                            "Detected recursions in the usage of quantifiers."
                        );
                        visited_func_symbols.insert(*identifier);

                        let inlined_body: &Stmt = function_body;
                        let mut stmts_of_inlined_body: Vec<Stmt> =
                            inlined_body.get_stmts().unwrap().clone();

                        // Substitute parameters with arguments in the function body.
                        if let Some(parameters) = self.symbol_table.lookup_parameters(*identifier) {
                            // Create decl statements of parameters.
                            let mut param_decls: Vec<Stmt> = parameters
                                .iter()
                                .zip(arguments.iter())
                                .map(|(param, arg)| {
                                    Stmt::decl(
                                        Expr::symbol_expression(*param, arg.typ().clone()),
                                        None,
                                        *arg.location(),
                                    )
                                })
                                .collect();

                            // Create assignment statements from arguments to parameters.
                            let mut param_assigs: Vec<Stmt> = parameters
                                .iter()
                                .zip(arguments.iter())
                                .map(|(param, arg)| {
                                    Stmt::assign(
                                        Expr::symbol_expression(*param, arg.typ().clone()),
                                        arg.clone(),
                                        *arg.location(),
                                    )
                                })
                                .collect();

                            // Prepend the assignments to stmts_of_inlined_body
                            param_decls.append(&mut param_assigs);
                            param_decls.append(&mut stmts_of_inlined_body);
                            stmts_of_inlined_body = param_decls;
                        }

                        let count_return: usize = stmts_of_inlined_body
                            .clone()
                            .iter()
                            .map(|stmt: &Stmt| Self::count_return_stmts(stmt))
                            .sum();
                        // The function is a void function, we safely ignore it.
                        if count_return == 0 {
                            return None;
                        }
                        // For simplicity, we currently only handle cases with one return statement.
                        assert_eq!(count_return, 1);

                        // Make labels in the inlined body unique.
                        let suffix = format!("_{}", suffix_count);
                        stmts_of_inlined_body = stmts_of_inlined_body
                            .iter()
                            .map(|stmt| Self::append_suffix_to_stmt(stmt, &suffix))
                            .collect();

                        // Replace all return stmts with symbol expressions.
                        let end_label: InternedString =
                            format!("KANI_quantifier_end{suffix}").into();
                        let mut end_stmt = None;
                        stmts_of_inlined_body = stmts_of_inlined_body
                            .iter()
                            .map(|stmt| {
                                Self::rewrite_return_stmt_with_goto(stmt, &mut end_stmt, &end_label)
                            })
                            .collect();
                        stmts_of_inlined_body
                            .push(Stmt::skip(*expr.location()).with_label(end_label));
                        stmts_of_inlined_body
                            .push(Stmt::code_expression(end_stmt.unwrap(), *expr.location()));

                        // Recursively inline function calls in the function body.
                        let res = self.inline_function_calls_in_expr(
                            &Expr::statement_expression(
                                stmts_of_inlined_body,
                                expr.typ().clone(),
                                *expr.location(),
                            ),
                            visited_func_symbols,
                            suffix_count,
                        );

                        visited_func_symbols.remove(identifier);

                        return res;
                    } else {
                        unreachable!()
                    }
                }
            }
            // Recursively inline function calls in ops.
            ExprValue::BinOp { op, lhs, rhs } => {
                return Some(
                    self.inline_function_calls_in_expr(lhs, visited_func_symbols, suffix_count)
                        .unwrap()
                        .binop(
                            *op,
                            self.inline_function_calls_in_expr(
                                rhs,
                                visited_func_symbols,
                                suffix_count,
                            )
                            .unwrap(),
                        ),
                );
            }
            ExprValue::StatementExpression { statements, location: _ } => {
                let inlined_stmts: Vec<Stmt> = statements
                    .iter()
                    .filter_map(|stmt| {
                        self.inline_function_calls_in_stmt(stmt, visited_func_symbols, suffix_count)
                    })
                    .collect();
                return Some(Expr::statement_expression(
                    inlined_stmts,
                    expr.typ().clone(),
                    *expr.location(),
                ));
            }
            _ => {}
        }
        Some(expr.clone())
    }

    /// Recursively inline all function calls in `stmt`.
    /// `visited_func_symbols` contain all function symbols in the stack.
    /// `suffix_count` is used to make inlined labels unique.
    fn inline_function_calls_in_stmt(
        &self,
        stmt: &Stmt,
        visited_func_symbols: &mut HashSet<InternedString>,
        suffix_count: &mut u16,
    ) -> Option<Stmt> {
        match stmt.body() {
            StmtBody::Expression(expr) => self
                .inline_function_calls_in_expr(expr, visited_func_symbols, suffix_count)
                .map(|inlined_expr| Stmt::code_expression(inlined_expr, *expr.location())),
            StmtBody::Assign { lhs, rhs } => self
                .inline_function_calls_in_expr(rhs, visited_func_symbols, suffix_count)
                .map(|inlined_rhs| Stmt::assign(lhs.clone(), inlined_rhs, *stmt.location())),
            StmtBody::Block(stmts) => {
                let inlined_block = stmts
                    .iter()
                    .filter_map(|s| {
                        self.inline_function_calls_in_stmt(s, visited_func_symbols, suffix_count)
                    })
                    .collect();
                Some(Stmt::block(inlined_block, *stmt.location()))
            }
            StmtBody::Label { label, body } => {
                match self.inline_function_calls_in_stmt(body, visited_func_symbols, suffix_count) {
                    None => Some(Stmt::skip(*stmt.location()).with_label(*label)),
                    Some(inlined_body) => Some(inlined_body.with_label(*label)),
                }
            }
            StmtBody::Switch { control, cases, default } => {
                // Inline function calls in the discriminant expression
                let inlined_control = self
                    .inline_function_calls_in_expr(control, visited_func_symbols, suffix_count)
                    .unwrap_or_else(|| control.clone());

                // Inline function calls in each case
                let inlined_cases: Vec<_> = cases
                    .iter()
                    .map(|sc| {
                        let inlined_stmt = self
                            .inline_function_calls_in_stmt(
                                sc.body(),
                                visited_func_symbols,
                                suffix_count,
                            )
                            .unwrap_or_else(|| sc.body().clone());
                        SwitchCase::new(sc.case().clone(), inlined_stmt)
                    })
                    .collect();

                // Inline function calls in the default case, if it exists
                let inlined_default = default.as_ref().map(|stmt| {
                    self.inline_function_calls_in_stmt(stmt, visited_func_symbols, suffix_count)
                        .unwrap_or_else(|| stmt.clone())
                });

                // Construct the new switch statement
                Some(Stmt::switch(
                    inlined_control,
                    inlined_cases,
                    inlined_default,
                    *stmt.location(),
                ))
            }
            StmtBody::While { .. } | StmtBody::For { .. } => {
                unimplemented!()
            }
            _ => Some(stmt.clone()),
        }
    }
}

/// Mutators
impl GotocCtx<'_> {
    pub fn set_current_fn(&mut self, instance: Instance, body: &Body) {
        self.current_fn = Some(CurrentFnCtx::new(instance, self, body));
    }

    pub fn reset_current_fn(&mut self) {
        self.current_fn = None;
    }

    pub fn next_global_name(&mut self) -> String {
        let c = self.global_var_count;
        self.global_var_count += 1;
        format!("{}::global::{c}::", self.full_crate_name())
    }

    pub fn next_check_id(&mut self) -> String {
        // check id is KANI_CHECK_ID_<crate_name>_<counter>
        let c = self.global_checks_count;
        self.global_checks_count += 1;
        format!("KANI_CHECK_ID_{}_{c}", self.full_crate_name)
    }
}

impl<'tcx> LayoutOfHelpers<'tcx> for GotocCtx<'tcx> {
    type LayoutOfResult = TyAndLayout<'tcx>;

    #[inline]
    fn handle_layout_err(&self, err: LayoutError<'tcx>, span: Span, ty: Ty<'tcx>) -> ! {
        span_bug!(span, "failed to get layout for `{}`: {}", ty, err)
    }
}

impl<'tcx> HasTypingEnv<'tcx> for GotocCtx<'tcx> {
    fn typing_env(&self) -> ty::TypingEnv<'tcx> {
        ty::TypingEnv::fully_monomorphized()
    }
}

impl<'tcx> HasTyCtxt<'tcx> for GotocCtx<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }
}

impl HasDataLayout for GotocCtx<'_> {
    fn data_layout(&self) -> &TargetDataLayout {
        self.tcx.data_layout()
    }
}

/// Implement error handling for extracting function ABI information.
impl<'tcx> FnAbiOfHelpers<'tcx> for GotocCtx<'tcx> {
    type FnAbiOfResult = &'tcx FnAbi<'tcx, Ty<'tcx>>;

    #[inline]
    fn handle_fn_abi_err(
        &self,
        err: FnAbiError<'tcx>,
        span: Span,
        fn_abi_request: FnAbiRequest<'tcx>,
    ) -> ! {
        if let FnAbiError::Layout(LayoutError::SizeOverflow(_)) = err {
            self.tcx.dcx().emit_fatal(respan(span, err))
        } else {
            match fn_abi_request {
                FnAbiRequest::OfFnPtr { sig, extra_args } => {
                    span_bug!(
                        span,
                        "Error: {err:?}\n while running `fn_abi_of_fn_ptr. ({sig}, {extra_args:?})`",
                    );
                }
                FnAbiRequest::OfInstance { instance, extra_args } => {
                    span_bug!(
                        span,
                        "Error: {err:?}\n while running `fn_abi_of_instance. ({instance}, {extra_args:?})`",
                    );
                }
            }
        }
    }
}
