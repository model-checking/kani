// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functions related to codegenning MIR functions into gotoc

use crate::context::metadata::HarnessMetadata;
use crate::GotocCtx;
use cbmc::goto_program::{Expr, Stmt, Symbol};
use rustc_ast::ast;
use rustc_middle::mir::{HasLocalDecls, Local};
use rustc_middle::ty::{self, Instance, TyS};
use tracing::{debug, warn};

/// Utility to skip functions that can't currently be successfully codgenned.
impl<'tcx> GotocCtx<'tcx> {
    fn should_skip_current_fn(&self) -> bool {
        match self.current_fn().readable_name() {
            // https://github.com/model-checking/rmc/issues/202
            "fmt::ArgumentV1::<'a>::as_usize" => true,
            // https://github.com/model-checking/rmc/issues/204
            name if name.ends_with("__getit") => true,
            // https://github.com/model-checking/rmc/issues/281
            name if name.starts_with("bridge::client") => true,
            // https://github.com/model-checking/rmc/issues/282
            "bridge::closure::Closure::<'a, A, R>::call" => true,
            // Generators
            name if name.starts_with("<std::future::from_generator::GenFuture<T>") => true,
            name if name.contains("reusable_box::ReusableBoxFuture") => true,
            "tokio::sync::Semaphore::acquire_owned::{closure#0}" => true,
            _ => false,
        }
    }
}

/// Codegen MIR functions into gotoc
impl<'tcx> GotocCtx<'tcx> {
    fn codegen_declare_variables(&mut self) {
        let mir = self.current_fn().mir();
        let ldecls = mir.local_decls();
        ldecls.indices().for_each(|lc| {
            if Some(lc) == mir.spread_arg {
                // We have already added this local in the function prelude, so
                // skip adding it again here.
                return;
            }
            let base_name = self.codegen_var_base_name(&lc);
            let name = self.codegen_var_name(&lc);
            let ldata = &ldecls[lc];
            let t = self.monomorphize(ldata.ty);
            let t = self.codegen_ty(t);
            let loc = self.codegen_span(&ldata.source_info.span);
            let sym =
                Symbol::variable(name, base_name, t, self.codegen_span(&ldata.source_info.span));
            let sym_e = sym.to_expr();
            self.symbol_table.insert(sym);

            // Index 0 represents the return value, which does not need to be
            // declared in the first block
            if lc.index() < 1 || lc.index() > mir.arg_count {
                self.current_fn_mut().push_onto_block(Stmt::decl(sym_e, None, loc));
            }
        });
    }

    pub fn codegen_function(&mut self, instance: Instance<'tcx>) {
        self.set_current_fn(instance);
        let name = self.current_fn().name();
        let old_sym = self.symbol_table.lookup(&name).unwrap();
        if old_sym.is_function_definition() {
            warn!("Double codegen of {:?}", old_sym);
        } else if self.should_skip_current_fn() {
            debug!("Skipping function {}", self.current_fn().readable_name());
            let loc = self.codegen_span(&self.current_fn().mir().span);
            let body = Stmt::assert_false(
                &format!(
                    "The function {} is not currently supported by RMC",
                    self.current_fn().readable_name()
                ),
                loc,
            );
            self.symbol_table.update_fn_declaration_with_definition(&name, body);
        } else {
            assert!(old_sym.is_function());
            let mir = self.current_fn().mir();
            self.print_instance(instance, mir);
            let labels = self
                .current_fn()
                .mir()
                .basic_blocks()
                .indices()
                .map(|bb| format!("{:?}", bb))
                .collect();
            self.current_fn_mut().set_labels(labels);
            self.codegen_function_prelude();
            self.codegen_declare_variables();

            mir.basic_blocks().iter_enumerated().for_each(|(bb, bbd)| self.codegen_block(bb, bbd));

            let loc = self.codegen_span(&mir.span);
            let stmts = self.current_fn_mut().extract_block();
            let body = Stmt::block(stmts, loc);
            self.symbol_table.update_fn_declaration_with_definition(&name, body);

            self.handle_rmctool_attributes();
        }
        self.reset_current_fn();
    }

    /// MIR functions have a `spread_arg` field that specifies whether the
    /// final argument to the function is "spread" at the LLVM/codegen level
    /// from a tuple into its individual components. (Used for the "rust-
    /// call" ABI, necessary because dynamic trait closure cannot have an
    /// argument list in MIR that is both generic and variadic, so Rust
    /// allows a generic tuple).
    ///
    /// If `spread_arg` is Some, then the wrapped value is the local that is
    /// to be "spread"/untupled. However, the MIR function body itself expects
    /// the tuple instead of the individual components, so we need to generate
    /// a function prelude that _retuples_, that is, writes the components
    /// back to the tuple local for use in the body.
    ///
    /// See:
    /// https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/Determine.20untupled.20closure.20args.20from.20Instance.3F
    fn codegen_function_prelude(&mut self) {
        let mir = self.current_fn().mir();
        if mir.spread_arg.is_none() {
            // No special tuple argument, no work to be done.
            return;
        }
        let spread_arg = mir.spread_arg.unwrap();
        let spread_data = &mir.local_decls()[spread_arg];
        let loc = self.codegen_span(&spread_data.source_info.span);

        // When we codegen the function signature elsewhere, we will codegen the
        // untupled version. So, the tuple argument itself needs to have a
        // symbol declared for it outside of the function signature, we do that
        // here.
        let tup_typ = self.codegen_ty(self.monomorphize(spread_data.ty));
        let tup_sym = Symbol::variable(
            self.codegen_var_name(&spread_arg),
            self.codegen_var_base_name(&spread_arg),
            tup_typ.clone(),
            loc.clone(),
        );
        self.symbol_table.insert(tup_sym.clone());

        // Get the function signature from MIR, _before_ we untuple
        let fntyp = self.current_fn().instance().ty(self.tcx, ty::ParamEnv::reveal_all());
        let sig = match fntyp.kind() {
            ty::FnPtr(..) | ty::FnDef(..) => fntyp.fn_sig(self.tcx).skip_binder(),
            // Closures themselves will have their arguments already untupled,
            // see Zulip link above.
            ty::Closure(..) => unreachable!(
                "Unexpected `spread arg` set for closure, got: {:?}, {:?}",
                fntyp,
                self.current_fn().readable_name()
            ),
            _ => unreachable!(
                "Expected function type for `spread arg` prelude, got: {:?}, {:?}",
                fntyp,
                self.current_fn().readable_name()
            ),
        };

        // Now that we have the tuple, write the individual component locals
        // back to it as a GotoC struct.
        let tupe = sig.inputs().last().unwrap();
        let args: Vec<&TyS<'tcx>> = match tupe.kind() {
            ty::Tuple(substs) => substs.iter().map(|s| s.expect_ty()).collect(),
            _ => unreachable!("a function's spread argument must be a tuple"),
        };

        // Convert each arg to a GotoC expression.
        let mut arg_exprs = Vec::new();
        let starting_idx = sig.inputs().len();
        for (arg_i, arg_t) in args.iter().enumerate() {
            // The components come at the end, so offset by the untupled length.
            let lc = Local::from_usize(arg_i + starting_idx);
            let (name, base_name) = self.codegen_spread_arg_name(&lc);
            let sym = Symbol::variable(name, base_name, self.codegen_ty(arg_t), loc.clone());
            self.symbol_table.insert(sym.clone());
            arg_exprs.push(sym.to_expr());
        }

        // Finally, combine the expression into a struct.
        let tuple_expr = Expr::struct_expr_from_values(tup_typ, arg_exprs, &self.symbol_table)
            .with_location(loc.clone());
        self.current_fn_mut().push_onto_block(Stmt::decl(tup_sym.to_expr(), Some(tuple_expr), loc));
    }

    pub fn declare_function(&mut self, instance: Instance<'tcx>) {
        debug!("declaring {}; {:?}", instance, instance);
        self.set_current_fn(instance);
        self.ensure(&self.current_fn().name(), |ctx, fname| {
            let mir = ctx.current_fn().mir();
            Symbol::function(
                fname,
                ctx.fn_typ(),
                None,
                Some(ctx.current_fn().readable_name()),
                ctx.codegen_span(&mir.span),
            )
        });
        self.reset_current_fn();
    }

    /// This updates the goto context with any information that should be accumulated from a function's
    /// attributes.
    ///
    /// Currently, this is only proof harness annotations.
    /// i.e. `#[rmc::proof]` (which rmc_macros translates to `#[rmctool::proof]` for us to handle here)
    fn handle_rmctool_attributes(&mut self) {
        let instance = self.current_fn().instance();

        for attr in self.tcx.get_attrs(instance.def_id()) {
            match rmctool_attr_name(attr).as_deref() {
                Some("proof") => self.handle_rmctool_proof(),
                _ => {}
            }
        }
    }

    /// Update `self` (the goto context) to add the current function as a listed proof harness
    fn handle_rmctool_proof(&mut self) {
        let current_fn = self.current_fn();
        let pretty_name = current_fn.readable_name().to_owned();
        let mangled_name = current_fn.name();
        let loc = self.codegen_span(&current_fn.mir().span);

        let harness =
            HarnessMetadata { pretty_name, mangled_name, original_file: loc.filename().unwrap() };

        self.proof_harnesses.push(harness);
    }
}

/// If the attribute is named `rmctool::name`, this extracts `name`
fn rmctool_attr_name(attr: &ast::Attribute) -> Option<String> {
    match &attr.kind {
        ast::AttrKind::Normal(ast::AttrItem { path: ast::Path { segments, .. }, .. }, _)
            if segments.len() == 2 && segments[0].ident.as_str() == "rmctool" =>
        {
            Some(segments[1].ident.as_str().to_string())
        }
        _ => None,
    }
}
