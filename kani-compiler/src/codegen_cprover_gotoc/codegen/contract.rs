// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functions related to codegenning code contracts from MIR into gotoc
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{Contract, Expr, Spec, Symbol};
use rustc_ast::NestedMetaItem;
use rustc_middle::mir::Local;
use rustc_middle::ty::{self, FnSig};

impl<'tcx> GotocCtx<'tcx> {
    /// Transforms current function's arguments into expressions
    fn extract_function_arguments(&mut self, sig: FnSig<'tcx>) -> Vec<Expr> {
        let args = sig.inputs();
        args.iter()
            .enumerate()
            .map(|(i, t)| {
                let lc = Local::from_usize(i + 1); // The zeroth index (local variable) is reserved for the return value of the function. Hence, function argument index starts from one.
                let ident = self.codegen_var_name(&lc);
                let typ = self.codegen_ty(*t);
                Expr::symbol_expression(ident, typ)
            })
            .collect()
    }

    /// Transforms current function's return value into an expression
    fn extract_function_return_value(&mut self, sig: FnSig<'tcx>) -> Expr {
        let rt = sig.output();
        let lc = Local::from_usize(0); // The zeroth index (local variable) is reserved for the return value of the function.
        let ident = self.codegen_var_name(&lc);
        let typ = self.codegen_ty(rt);
        Expr::symbol_expression(ident, typ)
    }

    /// In order to keep the contract symbol self-contained, every function contract clause
    /// such as `requires`, `ensures`, etc. is wrapped into a lambda expression.
    /// The binding variables of these lambda expressions are -
    ///  1) the return value of the function as the first variable, followed by,
    ///  2) the list of function arguments (in order).
    fn get_lambda_binding_variables(&mut self, sig: FnSig<'tcx>) -> Vec<Expr> {
        let mut vars = vec![];
        let ret = self.extract_function_return_value(sig);
        vars.push(ret);
        let args = self.extract_function_arguments(sig);
        vars.extend(args);
        vars
    }

    /// Generates a specification containing a lambda expression with "true" as its body
    fn spec_true(&mut self, sig: FnSig<'tcx>) -> Spec {
        let bv = self.get_lambda_binding_variables(sig);
        let mir = self.current_fn().mir();
        let loc = self.codegen_span(&mir.span);
        Spec::new(bv, Expr::bool_true(), loc)
    }

    /// Generates a new contract symbol and adds it to the symbol table.
    /// See <https://github.com/diffblue/cbmc/pull/6799> for further details about the contract symbol.
    /// The name of the contract symbol should be set to "contract::<function-name>".
    /// The type field of the contract symbol contains the `#spec_requires`, `#spec_ensures`, and `#spec_assigns` fields
    ///     for specifying the preconditions, postconditions, and the modifies (assigns/write) set of the function respectively.
    /// The contract symbol serves as the entry point for CBMC to check the contract.
    /// Since we want CBMC to only check the modifies clause, we set the other expected fields -
    ///     `#spec_requires` and `#spec_ensures` to "true".
    pub fn codegen_modifies_clause(&mut self, args: Vec<NestedMetaItem>) {
        let mir = self.current_fn().mir();
        let loc = self.codegen_span(&mir.span);
        let sig = self.current_fn().sig();
        let sig = self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig);
        let typ = self.fn_typ();

        let name = format!("contract::{}", self.current_fn().name()); // name of the contract symbol

        // Transform comma-separated "targets" (variables) from the modifies clause into specifications containing lambda expressions
        let spec_args = args
            .iter()
            .filter_map(|a| {
                let bv = self.get_lambda_binding_variables(sig);
                let ident = a.meta_item().unwrap().path.segments[0].ident;
                let basename = &ident.to_string();
                match self.lookup_local_decl_by_name(basename) {
                    // lookup the symbol for the argument in the symbol table
                    Some(sym) => {
                        let expr = Expr::symbol_expression(sym.name, sym.typ.clone());
                        Some(Spec::new(bv, expr, loc))
                    }
                    None => {
                        self.tcx.sess.span_err(
                            mir.span,
                            format!("Symbol {} not supported inside modifies clauses.", basename),
                        );
                        None
                    }
                }
            })
            .collect();
        let contract = Contract::function_contract(
            vec![self.spec_true(sig)],
            vec![self.spec_true(sig)],
            spec_args,
        );
        let sym = Symbol::contract(name, typ, contract, loc);
        self.symbol_table.insert(sym);
    }
}
