// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functions related to codegenning MIR functions into gotoc

use crate::codegen_cprover_gotoc::codegen::PropertyClass;
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{Expr, Stmt, Symbol};
use cbmc::InternString;
use kani_metadata::HarnessMetadata;
use rustc_ast::ast;
use rustc_ast::{Attribute, LitKind};
use rustc_middle::mir::{HasLocalDecls, Local};
use rustc_middle::ty::{self, Instance};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::iter::FromIterator;
use tracing::{debug, info_span};

/// Utility to skip functions that can't currently be successfully codgenned.
impl<'tcx> GotocCtx<'tcx> {
    fn should_skip_current_fn(&self) -> bool {
        match self.current_fn().readable_name() {
            // https://github.com/model-checking/kani/issues/202
            "fmt::ArgumentV1::<'a>::as_usize" => true,
            // https://github.com/model-checking/kani/issues/282
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
                Symbol::variable(name, base_name, t, self.codegen_span(&ldata.source_info.span))
                    .with_is_hidden(!ldata.is_user_variable());
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

        let _trace_span =
            info_span!("CodegenFunction", name = self.current_fn().readable_name()).entered();
        if old_sym.is_function_definition() {
            tracing::info!("Double codegen of {:?}", old_sym);
        } else if self.should_skip_current_fn() {
            debug!("Skipping function {}", self.current_fn().readable_name());
            let body = self.codegen_fatal_error(
                PropertyClass::UnsupportedConstruct,
                &GotocCtx::unsupported_msg(
                    &(String::from("The function ") + self.current_fn().readable_name()),
                    None,
                ),
                Some(self.current_fn().mir().span),
            );
            self.symbol_table.update_fn_declaration_with_definition(&name, body);
        } else {
            assert!(old_sym.is_function());
            let mir = self.current_fn().mir();
            self.print_instance(instance, mir);
            self.codegen_function_prelude();
            self.codegen_declare_variables();

            mir.basic_blocks().iter_enumerated().for_each(|(bb, bbd)| self.codegen_block(bb, bbd));

            let loc = self.codegen_span(&mir.span);
            let stmts = self.current_fn_mut().extract_block();
            let body = Stmt::block(stmts, loc);
            self.symbol_table.update_fn_declaration_with_definition(&name, body);

            self.handle_kanitool_attributes();
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
    /// <https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/Determine.20untupled.20closure.20args.20from.20Instance.3F>
    fn codegen_function_prelude(&mut self) {
        let mir = self.current_fn().mir();
        if mir.spread_arg.is_none() {
            // No special tuple argument, no work to be done.
            return;
        }
        let spread_arg = mir.spread_arg.unwrap();
        let spread_data = &mir.local_decls()[spread_arg];
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
        let tup_typ = self.codegen_ty(self.monomorphize(spread_data.ty));

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
                    .with_is_hidden(false);
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
            Expr::struct_expr(tup_typ.clone(), marshalled_tuple_fields, &self.symbol_table)
                .with_location(loc);
        self.declare_variable(
            self.codegen_var_name(&spread_arg),
            self.codegen_var_base_name(&spread_arg),
            tup_typ,
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

    /// This updates the goto context with any information that should be accumulated from a function's
    /// attributes.
    ///
    /// Handle all attributes i.e. `#[kani::x]` (which kani_macros translates to `#[kanitool::x]` for us to handle here)
    fn handle_kanitool_attributes(&mut self) {
        let all_attributes = self.tcx.get_attrs_unchecked(self.current_fn().instance().def_id());
        let (proof_attributes, other_attributes) = partition_kanitool_attributes(all_attributes);
        if proof_attributes.is_empty() && !other_attributes.is_empty() {
            self.tcx.sess.span_err(
                other_attributes[0].1.span,
                format!(
                    "The {} attribute also requires the '#[kani::proof]' attribute",
                    other_attributes[0].0
                )
                .as_str(),
            );
            return;
        }
        if proof_attributes.len() > 1 {
            // No return because this only requires a warning
            self.tcx.sess.span_warn(proof_attributes[0].span, "Only one '#[kani::proof]' allowed");
        }
        if !proof_attributes.is_empty() {
            self.create_proof_harness(other_attributes);
        }
    }

    /// Create the proof harness struct using the handler methods for various attributes
    fn create_proof_harness(&mut self, other_attributes: Vec<(String, &Attribute)>) {
        let mut harness = self.default_kanitool_proof();
        for attr in other_attributes.iter() {
            match attr.0.as_str() {
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
            original_file: loc.filename().unwrap(),
            original_line: loc.line().unwrap().to_string(),
            unwind_value: None,
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
                return;
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
}

/// If the attribute is named `kanitool::name`, this extracts `name`
fn kanitool_attr_name(attr: &ast::Attribute) -> Option<String> {
    match &attr.kind {
        ast::AttrKind::Normal(ast::AttrItem { path: ast::Path { segments, .. }, .. }, _)
            if (!segments.is_empty()) && segments[0].ident.as_str() == "kanitool" =>
        {
            Some(segments[1].ident.as_str().to_string())
        }
        _ => None,
    }
}

/// Partition all the attributes into two buckets, proof_attributes and other_attributes
fn partition_kanitool_attributes(
    all_attributes: &[Attribute],
) -> (Vec<&Attribute>, Vec<(String, &Attribute)>) {
    let mut proof_attributes = vec![];
    let mut other_attributes = vec![];

    for attr in all_attributes {
        // Get the string the appears after "kanitool::" in each attribute string.
        // Ex - "proof" | "unwind" etc.
        if let Some(attribute_string) = kanitool_attr_name(attr).as_deref() {
            if attribute_string == "proof" {
                proof_attributes.push(attr);
            } else {
                other_attributes.push((attribute_string.to_string(), attr));
            }
        }
    }

    (proof_attributes, other_attributes)
}

/// Extracts the integer value argument from the attribute provided
/// For example, `unwind(8)` return `Some(8)`
fn extract_integer_argument(attr: &Attribute) -> Option<u128> {
    // Vector of meta items , that contain the arguments given the attribute
    let attr_args = attr.meta_item_list()?;
    // Only extracts one integer value as argument
    if attr_args.len() == 1 {
        let x = attr_args[0].literal()?;
        match x.kind {
            LitKind::Int(y, ..) => Some(y),
            _ => None,
        }
    }
    // Return none if there are no attributes or if there's too many attributes
    else {
        None
    }
}
