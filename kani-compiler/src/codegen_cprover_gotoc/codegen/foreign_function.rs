// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module implements foreign function handling.
//!
//! Kani currently only support CBMC built-in functions that are declared in the `cprover_bindings`
//! crate, and allocation functions defined in `kani_lib.c`.
//!
//! All other functions will be replaced by an unimplemented check, due to current issues with
//! linking and usability unless unstable C-FFI support is enabled.
use std::collections::HashSet;

use crate::codegen_cprover_gotoc::codegen::PropertyClass;
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::unwrap_or_return_codegen_unimplemented_stmt;
use cbmc::goto_program::{Expr, Location, Stmt, Symbol, Type};
use cbmc::{InternString, InternedString};
use lazy_static::lazy_static;
use stable_mir::abi::{CallConvention, PassMode};
use stable_mir::mir::mono::Instance;
use stable_mir::mir::Place;
use stable_mir::ty::{RigidTy, TyKind};
use stable_mir::CrateDef;
use tracing::{debug, trace};

lazy_static! {
    /// The list of Rust allocation functions that are declared in the `core::alloc` module
    /// but defined by each backend.
    /// For our `goto-program` backend, these functions are defined inside `kani_lib.c`.
    /// For now, we blindly trust that the definitions in `kani_lib.c` are kept in sync with the
    /// declarations from the standard library, provided here:
    /// <https://stdrs.dev/nightly/x86_64-unknown-linux-gnu/alloc/alloc/index.html>
    static ref RUST_ALLOC_FNS: HashSet<InternedString> = {
        HashSet::from([
            "__rust_alloc".into(),
            "__rust_alloc_zeroed".into(),
            "__rust_dealloc".into(),
            "__rust_realloc".into(),
            "__KANI_pointer_object".into(),
            "__KANI_pointer_offset".into(),
        ])
    };
}

impl<'tcx> GotocCtx<'tcx> {
    /// Generate the symbol and symbol table entry for foreign items.
    ///
    /// CBMC built-in functions that are supported by Kani are always added to the symbol table, and
    /// this function will return them.
    ///
    /// For other foreign items, we declare a shim and add to the list of foreign shims to be
    /// handled later.
    pub fn codegen_foreign_fn(&mut self, instance: Instance) -> &Symbol {
        debug!(?instance, "codegen_foreign_function");
        let fn_name = self.symbol_name_stable(instance).intern();
        if self.symbol_table.contains(fn_name) {
            // Symbol has been added (either a built-in CBMC function or a Rust allocation function).
            self.symbol_table.lookup(fn_name).unwrap()
        } else if RUST_ALLOC_FNS.contains(&fn_name)
            || (self.is_cffi_enabled() && instance.fn_abi().unwrap().conv == CallConvention::C)
        {
            // Add a Rust alloc lib function as is declared by core.
            // When C-FFI feature is enabled, we just trust the rust declaration.
            // TODO: Add proper casting and clashing definitions check.
            // https://github.com/model-checking/kani/issues/1350
            // https://github.com/model-checking/kani/issues/2426
            self.ensure(fn_name, |gcx, _| {
                let typ = gcx.codegen_ffi_type(instance);
                Symbol::function(fn_name, typ, None, instance.name(), Location::none())
                    .with_is_extern(true)
            })
        } else {
            let shim_name = format!("{fn_name}_ffi_shim");
            trace!(?shim_name, "codegen_foreign_function");
            self.ensure(&shim_name, |gcx, _| {
                // Generate a shim with an unsupported C-FFI error message.
                let typ = gcx.codegen_ffi_type(instance);
                Symbol::function(
                    &shim_name,
                    typ,
                    Some(gcx.codegen_ffi_shim(shim_name.as_str().into(), instance)),
                    instance.name(),
                    Location::none(),
                )
            })
        }
    }

    /// Generate a function call to a foreign function by potentially casting arguments and return value, since
    /// the external function definition may not match exactly its Rust declaration.
    /// See <https://github.com/model-checking/kani/issues/1350#issuecomment-1192036619> for more details.
    pub fn codegen_foreign_call(
        &mut self,
        fn_expr: Expr,
        args: Vec<Expr>,
        ret_place: &Place,
        loc: Location,
    ) -> Stmt {
        let expected_args = fn_expr
            .typ()
            .parameters()
            .unwrap()
            .iter()
            .zip(args)
            .map(|(param, arg)| arg.cast_to(param.typ().clone()))
            .collect::<Vec<_>>();
        let call_expr = fn_expr.call(expected_args);

        let ret_kind = self.place_ty_stable(ret_place).kind();
        if ret_kind.is_unit() || matches!(ret_kind, TyKind::RigidTy(RigidTy::Never)) {
            call_expr.as_stmt(loc)
        } else {
            let ret_expr = unwrap_or_return_codegen_unimplemented_stmt!(
                self,
                self.codegen_place_stable(ret_place)
            )
            .goto_expr;
            let ret_type = ret_expr.typ().clone();
            ret_expr.assign(call_expr.cast_to(ret_type), loc)
        }
    }

    /// Checks whether C-FFI has been enabled or not.
    /// When enabled, we blindly encode the function type as is.
    fn is_cffi_enabled(&self) -> bool {
        self.queries.args().unstable_features.contains(&"c-ffi".to_string())
    }

    /// Generate code for a foreign function shim.
    fn codegen_ffi_shim(&mut self, shim_name: InternedString, instance: Instance) -> Stmt {
        debug!(?shim_name, ?instance, sym=?self.symbol_table.lookup(shim_name), "generate_foreign_shim");

        let loc = self.codegen_span_stable(instance.def.span());
        let unsupported_check = self.codegen_ffi_unsupported(instance, loc);
        Stmt::block(vec![unsupported_check], loc)
    }

    /// Generate type for the given foreign instance.
    fn codegen_ffi_type(&mut self, instance: Instance) -> Type {
        let fn_name = self.symbol_name_stable(instance);
        let fn_abi = instance.fn_abi().unwrap();
        let loc = self.codegen_span_stable(instance.def.span());
        let params = fn_abi
            .args
            .iter()
            .enumerate()
            .filter(|&(_, arg)| (arg.mode != PassMode::Ignore))
            .map(|(idx, arg)| {
                let arg_name = format!("{fn_name}::param_{idx}");
                let base_name = format!("param_{idx}");
                let arg_type = self.codegen_ty_stable(arg.ty);
                let sym = Symbol::variable(&arg_name, &base_name, arg_type.clone(), loc)
                    .with_is_parameter(true);
                self.symbol_table.insert(sym);
                arg_type.as_parameter(Some(arg_name.into()), Some(base_name.into()))
            })
            .collect();
        let ret_type = self.codegen_ty_stable(fn_abi.ret.ty);

        if fn_abi.c_variadic {
            Type::variadic_code(params, ret_type)
        } else {
            Type::code(params, ret_type)
        }
    }

    /// Kani does not currently support FFI functions except for built-in CBMC functions.
    ///
    /// This will behave like `codegen_unimplemented_stmt` but print a message that includes
    /// the name of the function not supported and the calling convention.
    fn codegen_ffi_unsupported(&mut self, instance: Instance, loc: Location) -> Stmt {
        let fn_name = &self.symbol_name_stable(instance);
        debug!(?fn_name, ?loc, "codegen_ffi_unsupported");

        // Save this occurrence so we can emit a warning in the compilation report.
        let entry = self.unsupported_constructs.entry("foreign function".into()).or_default();
        entry.push(loc);

        let call_conv = instance.fn_abi().unwrap().conv;
        let msg = format!("call to foreign \"{call_conv:?}\" function `{fn_name}`");
        let url = if call_conv == CallConvention::C {
            "https://github.com/model-checking/kani/issues/2423"
        } else {
            "https://github.com/model-checking/kani/issues/new/choose"
        };
        self.codegen_assert_assume(
            Expr::bool_false(),
            PropertyClass::UnsupportedConstruct,
            &GotocCtx::unsupported_msg(&msg, Some(url)),
            loc,
        )
    }
}
