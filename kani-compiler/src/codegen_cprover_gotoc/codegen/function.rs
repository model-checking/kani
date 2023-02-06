// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functions related to codegenning MIR functions into gotoc

use crate::codegen_cprover_gotoc::GotocCtx;
use crate::kani_middle::attributes::{extract_integer_argument, partition_kanitool_attributes};
use cbmc::goto_program::{Expr, Stmt, Symbol};
use cbmc::InternString;
use kani_metadata::{CbmcSolver, HarnessMetadata};
use kani_queries::UserInput;
use rustc_ast::{Attribute, MetaItemKind};
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::mir::traversal::reverse_postorder;
use rustc_middle::mir::{Body, HasLocalDecls, Local};
use rustc_middle::ty::layout::FnAbiOf;
use rustc_middle::ty::{self, Instance};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::iter::FromIterator;
use std::str::FromStr;
use tracing::{debug, debug_span};

/// Codegen MIR functions into gotoc
impl<'tcx> GotocCtx<'tcx> {
    /// Get the number of parameters that the current function expects.
    fn get_params_size(&self) -> usize {
        let sig = self.current_fn().sig();
        let sig = self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig);
        // we don't call [codegen_function_sig] because we want to get a bit more metainformation.
        sig.inputs().len()
    }

    /// Declare variables according to their index.
    /// - Index 0 represents the return value.
    /// - Indices [1, N] represent the function parameters where N is the number of parameters.
    /// - Indices that are greater than N represent local variables.
    fn codegen_declare_variables(&mut self) {
        let mir = self.current_fn().mir();
        let ldecls = mir.local_decls();
        let num_args = self.get_params_size();
        ldecls.indices().enumerate().for_each(|(idx, lc)| {
            if Some(lc) == mir.spread_arg {
                // We have already added this local in the function prelude, so
                // skip adding it again here.
                return;
            }
            let base_name = self.codegen_var_base_name(&lc);
            let name = self.codegen_var_name(&lc);
            let ldata = &ldecls[lc];
            let var_ty = self.monomorphize(ldata.ty);
            let var_type = self.codegen_ty(var_ty);
            let loc = self.codegen_span(&ldata.source_info.span);
            // Indices [1, N] represent the function parameters where N is the number of parameters.
            // Except that ZST fields are not included as parameters.
            let sym = Symbol::variable(
                name,
                base_name,
                var_type,
                self.codegen_span(&ldata.source_info.span),
            )
            .with_is_hidden(!ldata.is_user_variable())
            .with_is_parameter((idx > 0 && idx <= num_args) && !self.is_zst(var_ty));
            let sym_e = sym.to_expr();
            self.symbol_table.insert(sym);

            // Index 0 represents the return value, which does not need to be
            // declared in the first block
            if lc.index() < 1 || lc.index() > mir.arg_count {
                let init = self.codegen_default_initializer(&sym_e);
                self.current_fn_mut().push_onto_block(Stmt::decl(sym_e, init, loc));
            }
        });
    }

    pub fn codegen_function(&mut self, instance: Instance<'tcx>) {
        self.set_current_fn(instance);
        let name = self.current_fn().name();
        let old_sym = self.symbol_table.lookup(&name).unwrap();

        let _trace_span =
            debug_span!("CodegenFunction", name = self.current_fn().readable_name()).entered();
        if old_sym.is_function_definition() {
            debug!("Double codegen of {:?}", old_sym);
        } else {
            assert!(old_sym.is_function());
            let mir = self.current_fn().mir();
            self.print_instance(instance, mir);
            self.codegen_function_prelude();
            self.codegen_declare_variables();

            reverse_postorder(mir).for_each(|(bb, bbd)| self.codegen_block(bb, bbd));

            let loc = self.codegen_span(&mir.span);
            let stmts = self.current_fn_mut().extract_block();
            let body = Stmt::block(stmts, loc);
            self.symbol_table.update_fn_declaration_with_definition(&name, body);

            self.handle_kanitool_attributes();
            self.record_test_harness_metadata();
        }
        self.reset_current_fn();
    }

    /// Codegen changes required due to the function ABI.
    /// We currently untuple arguments for RustCall ABI where the `spread_arg` is set.
    fn codegen_function_prelude(&mut self) {
        let mir = self.current_fn().mir();
        if let Some(spread_arg) = mir.spread_arg {
            self.codegen_spread_arg(mir, spread_arg);
        }
    }

    /// MIR functions have a `spread_arg` field that specifies whether the
    /// final argument to the function is "spread" at the LLVM/codegen level
    /// from a tuple into its individual components. (Used for the "rust-
    /// call" ABI, necessary because the function traits and closures cannot have an
    /// argument list in MIR that is both generic and variadic, so Rust
    /// allows a generic tuple).
    ///
    /// These tuples are used in the MIR to invoke a shim, and it's used in the shim body.
    ///
    /// The `spread_arg` represents the the local variable that is to be "spread"/untupled.
    /// However, the function body itself may refer to the members of
    /// the tuple instead of the individual spread parameters, so we need to add to the
    /// function prelude code that _retuples_, that is, writes the arguments
    /// back to a local tuple that can be used in the body.
    ///
    /// See:
    /// <https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/Determine.20untupled.20closure.20args.20from.20Instance.3F>
    fn codegen_spread_arg(&mut self, mir: &Body<'tcx>, spread_arg: Local) {
        tracing::debug!(current=?self.current_fn, "codegen_spread_arg");
        let spread_data = &mir.local_decls()[spread_arg];
        let tup_ty = self.monomorphize(spread_data.ty);
        if self.is_zst(tup_ty) {
            // No need to spread a ZST since it will be ignored.
            return;
        }

        let loc = self.codegen_span(&spread_data.source_info.span);

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

        // When we codegen the function signature elsewhere, we will codegen the untupled version.
        // We then marshall the arguments into a local variable holding the expected tuple.
        // For a function with args f(a: t1, b: t2, c: t3), the tuple type will look like
        // ```
        //    struct T {
        //        0: t1,
        //        1: t2,
        //        2: t3,
        // }
        // ```
        // For e.g., in the test `tupled_closure.rs`, the tuple type looks like:
        // ```
        // struct _8098103865751214180
        // {
        //    unsigned long int 1;
        //    unsigned char 0;
        //    struct _3159196586427472662 2;
        // };
        // ```
        // Note how the compiler has reordered the fields to improve packing.
        let tup_type = self.codegen_ty(tup_ty);

        // We need to marshall the arguments into the tuple
        // The arguments themselves have been tacked onto the explicit function paramaters by
        // the code in `pub fn fn_typ(&mut self) -> Type {` in `typ.rs`.
        // By convention, they are given the names `spread<i>`.
        // For e.g., in the test `tupled_closure.rs`, the actual function looks like
        // ```
        // unsigned long int _RNvYNvCscgV8bIzQQb7_14tupled_closure1hINtNtNtCsaGHNm3cehi1_4core3ops8function2FnThjINtNtBH_6option6OptionNtNtNtBH_3num7nonzero12NonZeroUsizeEEE4callB4_(
        //        unsigned long int (*var_1)(unsigned char, unsigned long int, struct _3159196586427472662),
        //        unsigned char spread_2,
        //        unsigned long int spread_3,
        //        struct _3159196586427472662 spread_4) {
        //  struct _8098103865751214180 var_2={ .1=spread_3, .0=spread_2, .2=spread_4 };
        //  unsigned long int var_0=(_RNvCscgV8bIzQQb7_14tupled_closure1h)(var_2.0, var_2.1, var_2.2);
        //  return var_0;
        // }
        // ```

        let tupe = sig.inputs().last().unwrap();
        let args = match tupe.kind() {
            ty::Tuple(substs) => *substs,
            _ => unreachable!("a function's spread argument must be a tuple"),
        };
        let starting_idx = sig.inputs().len();
        let marshalled_tuple_fields =
            BTreeMap::from_iter(args.iter().enumerate().map(|(arg_i, arg_t)| {
                // The components come at the end, so offset by the untupled length.
                // This follows the naming convention defined in `typ.rs`.
                let lc = Local::from_usize(arg_i + starting_idx);
                let (name, base_name) = self.codegen_spread_arg_name(&lc);
                let sym = Symbol::variable(name, base_name, self.codegen_ty(arg_t), loc)
                    .with_is_hidden(false)
                    .with_is_parameter(!self.is_zst(arg_t));
                // The spread arguments are additional function paramaters that are patched in
                // They are to the function signature added in the `fn_typ` function.
                // But they were never added to the symbol table, which we currently do here.
                // https://github.com/model-checking/kani/issues/686 to track a better solution.
                self.symbol_table.insert(sym.clone());
                // As discussed above, fields are named like `0: t1`.
                // Follow that pattern for the marshalled data.
                // name:value map is resilliant to rustc reordering fields (see above)
                (arg_i.to_string().intern(), sym.to_expr())
            }));
        let marshalled_tuple_value =
            Expr::struct_expr(tup_type.clone(), marshalled_tuple_fields, &self.symbol_table)
                .with_location(loc);
        self.declare_variable(
            self.codegen_var_name(&spread_arg),
            self.codegen_var_base_name(&spread_arg),
            tup_type,
            Some(marshalled_tuple_value),
            loc,
        );
    }

    pub fn declare_function(&mut self, instance: Instance<'tcx>) {
        debug!("declaring {}; {:?}", instance, instance);
        self.set_current_fn(instance);
        debug!(krate = self.current_fn().krate().as_str());
        debug!(is_std = self.current_fn().is_std());
        self.ensure(&self.current_fn().name(), |ctx, fname| {
            let mir = ctx.current_fn().mir();
            Symbol::function(
                fname,
                ctx.fn_typ(),
                None,
                ctx.current_fn().readable_name(),
                ctx.codegen_span(&mir.span),
            )
        });
        self.reset_current_fn();
    }

    /// Check that if an item is tagged with a proof_attribute, it is a valid harness.
    fn check_proof_attribute(&self, def_id: DefId, proof_attributes: Vec<&Attribute>) {
        assert!(!proof_attributes.is_empty());
        let span = proof_attributes.first().unwrap().span;
        if proof_attributes.len() > 1 {
            self.tcx.sess.span_warn(proof_attributes[0].span, "Duplicate attribute");
        }

        if self.tcx.def_kind(def_id) != DefKind::Fn {
            self.tcx
                .sess
                .span_err(span, "The kani::proof attribute can only be applied to functions.");
        } else if self.tcx.generics_of(def_id).requires_monomorphization(self.tcx) {
            self.tcx
                .sess
                .span_err(span, "The proof attribute cannot be applied to generic functions.");
        } else {
            let instance = Instance::mono(self.tcx, def_id);
            if !self.fn_abi_of_instance(instance, ty::List::empty()).args.is_empty() {
                self.tcx
                    .sess
                    .span_err(span, "Functions used as harnesses can not have any arguments.");
            }
        }
    }

    pub fn is_proof_harness(&self, def_id: DefId) -> bool {
        let all_attributes = self.tcx.get_attrs_unchecked(def_id);
        let (proof_attributes, _) = partition_kanitool_attributes(all_attributes);
        !proof_attributes.is_empty()
    }

    /// Check that all attributes assigned to an item is valid.
    /// Errors will be added to the session. Invoke self.tcx.sess.abort_if_errors() to terminate
    /// the session in case of an error.
    pub fn check_attributes(&self, def_id: DefId) {
        let all_attributes = self.tcx.get_attrs_unchecked(def_id);
        let (proof_attributes, other_attributes) = partition_kanitool_attributes(all_attributes);
        if !proof_attributes.is_empty() {
            self.check_proof_attribute(def_id, proof_attributes);
        } else if !other_attributes.is_empty() {
            self.tcx.sess.span_err(
                other_attributes[0].1.span,
                format!(
                    "The {} attribute also requires the '#[kani::proof]' attribute",
                    other_attributes[0].0
                )
                .as_str(),
            );
        }
    }

    /// Does this `def_id` have `#[rustc_test_marker]`?
    pub fn is_test_harness_description(&self, def_id: DefId) -> bool {
        let attrs = self.tcx.get_attrs_unchecked(def_id);

        self.tcx.sess.contains_name(attrs, rustc_span::symbol::sym::rustc_test_marker)
    }
    /// Is this the closure inside of a test description const (i.e. macro expanded from a `#[test]`)?
    ///
    /// We're trying to detect the closure (`||`) inside code like:
    ///
    /// ```ignore
    /// #[rustc_test_marker]
    /// pub const check_2: test::TestDescAndFn = test::TestDescAndFn {
    ///     desc: ...,
    ///     testfn: test::StaticTestFn(|| test::assert_test_result(check_2())),
    /// };
    /// ```
    pub fn is_test_harness_closure(&self, def_id: DefId) -> bool {
        if !def_id.is_local() {
            return false;
        }

        let local_def_id = def_id.expect_local();
        let hir_id = self.tcx.hir().local_def_id_to_hir_id(local_def_id);

        // The parent item of the closure appears to reliably be the `const` declaration item.
        let parent_id = self.tcx.hir().get_parent_item(hir_id);
        let parent_def_id = parent_id.to_def_id();

        self.is_test_harness_description(parent_def_id)
    }

    /// We record test harness information in kani-metadata, just like we record
    /// proof harness information. This is used to support e.g. cargo-kani assess.
    ///
    /// Note that we do not actually spot the function that was annotated by `#[test]`
    /// but instead the closure that gets put into the "test description" that macro
    /// expands into. (See comment below) This ends up being preferrable, actually,
    /// as it add asserts for tests that return `Result` types.
    fn record_test_harness_metadata(&mut self) {
        let def_id = self.current_fn().instance().def_id();
        if self.is_test_harness_closure(def_id) {
            let loc = self.codegen_span(&self.current_fn().mir().span);
            self.test_harnesses.push(HarnessMetadata {
                pretty_name: self.current_fn().readable_name().to_owned(),
                mangled_name: self.current_fn().name(),
                crate_name: self.current_fn().krate(),
                original_file: loc.filename().unwrap(),
                original_start_line: loc.start_line().unwrap() as usize,
                original_end_line: loc.end_line().unwrap() as usize,
                solver: None,
                unwind_value: None,
                // We record the actual path after codegen before we dump the metadata into a file.
                goto_file: None,
            })
        }
    }

    /// This updates the goto context with any information that should be accumulated from a function's
    /// attributes.
    ///
    /// Handle all attributes i.e. `#[kani::x]` (which kani_macros translates to `#[kanitool::x]` for us to handle here)
    fn handle_kanitool_attributes(&mut self) {
        let def_id = self.current_fn().instance().def_id();
        let all_attributes = self.tcx.get_attrs_unchecked(def_id);
        let (proof_attributes, other_attributes) = partition_kanitool_attributes(all_attributes);
        if !proof_attributes.is_empty() {
            self.create_proof_harness(other_attributes);
        }
    }

    /// Create the proof harness struct using the handler methods for various attributes
    fn create_proof_harness(&mut self, other_attributes: Vec<(String, &Attribute)>) {
        let mut harness = self.default_kanitool_proof();
        for attr in other_attributes.iter() {
            match attr.0.as_str() {
                "solver" => self.handle_kanitool_solver(attr.1, &mut harness),
                "stub" => {
                    if !self.queries.get_stubbing_enabled() {
                        self.tcx.sess.span_warn(
                            attr.1.span,
                            "Stubbing is not enabled; attribute `kani::stub` will be ignored",
                        )
                    }
                }
                "unwind" => self.handle_kanitool_unwind(attr.1, &mut harness),
                _ => {
                    self.tcx.sess.span_err(
                        attr.1.span,
                        format!("Unsupported Annotation -> {}", attr.0.as_str()).as_str(),
                    );
                }
            }
        }
        self.proof_harnesses.push(harness);
    }

    /// Create the default proof harness for the current function
    fn default_kanitool_proof(&mut self) -> HarnessMetadata {
        let current_fn = self.current_fn();
        let pretty_name = current_fn.readable_name().to_owned();
        let mangled_name = current_fn.name();
        let loc = self.codegen_span(&current_fn.mir().span);

        HarnessMetadata {
            pretty_name,
            mangled_name,
            crate_name: current_fn.krate(),
            original_file: loc.filename().unwrap(),
            original_start_line: loc.start_line().unwrap() as usize,
            original_end_line: loc.end_line().unwrap() as usize,
            solver: None,
            unwind_value: None,
            // We record the actual path after codegen before we dump the metadata into a file.
            goto_file: None,
        }
    }

    /// Updates the proof harness with new unwind value
    fn handle_kanitool_unwind(&mut self, attr: &Attribute, harness: &mut HarnessMetadata) {
        // If some unwind value already exists, then the current unwind being handled is a duplicate
        if harness.unwind_value.is_some() {
            self.tcx.sess.span_err(attr.span, "Only one '#[kani::unwind]' allowed");
            return;
        }
        // Get Attribute value and if it's not none, assign it to the metadata
        match extract_integer_argument(attr) {
            None => {
                // There are no integers or too many arguments given to the attribute
                self.tcx
                    .sess
                    .span_err(attr.span, "Exactly one Unwind Argument as Integer accepted");
            }
            Some(unwind_integer_value) => {
                let val: Result<u32, _> = unwind_integer_value.try_into();
                if val.is_err() {
                    self.tcx
                        .sess
                        .span_err(attr.span, "Value above maximum permitted value - u32::MAX");
                    return;
                }
                harness.unwind_value = Some(val.unwrap());
            }
        }
    }

    /// Set the solver for this proof harness
    fn handle_kanitool_solver(&mut self, attr: &Attribute, harness: &mut HarnessMetadata) {
        // Make sure the solver is not already set
        if harness.solver.is_some() {
            self.tcx
                .sess
                .span_err(attr.span, "Only one '#[kani::solver]' attribute is allowed per harness");
            return;
        }
        harness.solver = self.extract_solver_argument(attr);
    }

    fn extract_solver_argument(&mut self, attr: &Attribute) -> Option<CbmcSolver> {
        // TODO: Argument validation should be done as part of the kani_macros crate
        // https://github.com/model-checking/kani/issues/2192
        const ATTRIBUTE: &str = "#[kani::solver]";
        let invalid_arg_err = |attr: &Attribute| {
            self.tcx.sess.span_err(
                attr.span,
                format!("Invalid argument to `{ATTRIBUTE}` attribute. Expecting one of the supported solvers, e.g. `kissat` or a SAT solver binary, e.g. `custom=\"sat-solver-binary\"`.")
            )
        };

        let attr_args = attr.meta_item_list().unwrap();
        if attr_args.len() != 1 {
            self.tcx.sess.span_err(
                attr.span,
                format!(
                    "The `{ATTRIBUTE}` attribute expects a single argument. Got {} arguments.",
                    attr_args.len()
                ),
            );
            return None;
        }
        let attr_arg = &attr_args[0];
        let meta_item = attr_arg.meta_item();
        if meta_item.is_none() {
            invalid_arg_err(attr);
            return None;
        }
        let meta_item = meta_item.unwrap();
        let ident = meta_item.ident().unwrap();
        let ident_str = ident.as_str();
        match &meta_item.kind {
            MetaItemKind::Word => {
                let solver = CbmcSolver::from_str(ident_str);
                match solver {
                    Ok(solver) => Some(solver),
                    Err(_) => {
                        self.tcx.sess.span_err(attr.span, format!("Unknown solver `{ident_str}`"));
                        None
                    }
                }
            }
            MetaItemKind::NameValue(lit) if ident_str == "custom" && lit.kind.is_str() => {
                //Some(CbmcSolver::Custom(meta_item.name_value_literal().unwrap().token_lit.symbol.to_string()))
                Some(CbmcSolver::Custom(lit.token_lit.symbol.to_string()))
            }
            _ => {
                invalid_arg_err(attr);
                None
            }
        }
    }
}
