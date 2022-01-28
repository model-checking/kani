// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// We can improve verification performance by conveying semantic information
/// about function pointer calls down to the underlying solver. In particular,
/// method calls through Rust dynamic trait objects use a vtable (virtual method
/// table) that stores method pointers for the concrete object type rather than
/// the shared abstract trait type.
///
/// This file build a map of virtual call sites to all of the possible trait
/// implementations that match that method and trait. We then can write out
/// this information as function pointer restrictions, improving verification
/// performance compared to heuristics that consider a wider set of possible
/// function pointer targets.
///
/// For the current CBMC implementation of function restrictions, see:
///     http://cprover.diffblue.com/md__home_travis_build_diffblue_cbmc_doc_architectural_restrict-function-pointer.html
use crate::GotocCtx;
use cbmc::goto_program::{Expr, Location, Stmt, Symbol, Type};
use cbmc::InternedString;
use cbmc::NO_PRETTY_NAME;
use kani_restrictions::{CallSite, PossibleMethodEntry, TraitDefinedMethod, VtableCtxResults};
use rustc_data_structures::stable_map::FxHashMap;
use tracing::debug;

/// This structure represents data about the vtable that we construct
/// Per trait, per method, which functions could virtual call sites
/// possibly refer to?
pub struct VtableCtx {
    // Option to actually enable restrictions
    pub emit_vtable_restrictions: bool,

    // Map: (normalized trait name, method index) -> possible implementations
    possible_methods: FxHashMap<TraitDefinedMethod, Vec<InternedString>>,

    // All sites where a virtual call takes place
    call_sites: Vec<CallSite>,

    // Internal tracing of index needed for call site wrappers
    call_site_global_idx: usize,
}

/// Constructor
impl VtableCtx {
    pub fn new(emit_vtable_restrictions: bool) -> Self {
        debug!("Restricting vtable function pointers? {:?}", emit_vtable_restrictions);
        Self {
            emit_vtable_restrictions,
            possible_methods: FxHashMap::default(),
            call_sites: Vec::new(),
            call_site_global_idx: 0,
        }
    }
}

/// Interface for codegen to add possible methods
impl VtableCtx {
    /// Add a possible implementation for a virtual method call.
    pub fn add_possible_method(
        &mut self,
        trait_name: InternedString,
        method: usize,
        imp: InternedString,
    ) {
        assert!(self.emit_vtable_restrictions);
        let key = TraitDefinedMethod { trait_name, vtable_idx: method };

        if let Some(possibilities) = self.possible_methods.get_mut(&key) {
            possibilities.push(imp);
        } else {
            self.possible_methods.insert(key, vec![imp]);
        }
    }

    /// The vtable index for drop
    pub fn drop_index() -> usize {
        rustc_middle::ty::COMMON_VTABLE_ENTRIES_DROPINPLACE
    }
}

/// Internal tracking helpers
impl VtableCtx {
    fn get_call_site_global_idx(&mut self) -> usize {
        assert!(self.emit_vtable_restrictions);
        self.call_site_global_idx += 1;
        self.call_site_global_idx
    }

    /// Add a given call site for a virtual function
    fn add_call_site(
        &mut self,
        trait_name: InternedString,
        method: usize,
        function_name: InternedString,
    ) {
        assert!(self.emit_vtable_restrictions);
        let site = CallSite {
            trait_method: TraitDefinedMethod { trait_name, vtable_idx: method },
            function_name: function_name,
        };
        self.call_sites.push(site);
    }
}

impl<'tcx> GotocCtx<'tcx> {
    /// Wrap a virtual call through a function pointer and restrict the
    /// possible targets.
    ///
    /// We need to wrap because for the current implemention, CBMC employs
    /// a hard-to-get-right naming scheme for restrictions: the call site is
    /// named for its index in  a given function. We don't have a good way to
    /// track _all_ function pointers within the function, so wrapping the call
    /// to a function that makes a single virtual function pointer call makes
    /// the naming unambiguous.
    ///
    /// This can be simplified if CBMC implemented label-based restrictions.
    /// Kani tracking: https://github.com/model-checking/kani/issues/651
    /// CBMC tracking: https://github.com/diffblue/cbmc/issues/6464
    pub fn virtual_call_with_restricted_fn_ptr(
        &mut self,
        trait_ty: Type,
        vtable_idx: usize,
        fn_ptr: Expr,
        args: Vec<Expr>,
    ) -> Expr {
        assert!(self.vtable_ctx.emit_vtable_restrictions);

        // Crate-based naming scheme for wrappers
        let full_crate_name = self.full_crate_name().to_string().replace("::", "_");
        let wrapper_name: InternedString = format!(
            "restricted_call_{}_{}",
            full_crate_name,
            self.vtable_ctx.get_call_site_global_idx()
        )
        .into();

        // We only have the Gotoc type, we need to normalize to match the MIR type.
        assert!(trait_ty.is_struct_tag());
        let mir_name =
            self.normalized_trait_name(self.type_map.get(&trait_ty.tag().unwrap()).unwrap());
        self.vtable_ctx.add_call_site(mir_name.into(), vtable_idx, wrapper_name);

        // Declare the wrapper's parameters
        let func_exp: Expr = fn_ptr.dereference();
        let fn_type = func_exp.typ().clone();
        let parameters: Vec<Symbol> = fn_type
            .parameters()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, parameter)| {
                let name = format!("{}_{}", wrapper_name, i);
                let param =
                    Symbol::variable(name.clone(), name, parameter.typ().clone(), Location::none());
                self.symbol_table.insert(param.clone());
                param
            })
            .collect();

        // Finish constructing the wrapper type
        let ret_typ = fn_type.return_type().unwrap().clone();
        let param_typs = parameters.clone().iter().map(|p| p.to_function_parameter()).collect();
        let new_typ = if fn_type.is_code() {
            Type::code(param_typs, ret_typ.clone())
        } else if fn_type.is_variadic_code() {
            Type::variadic_code(param_typs, ret_typ.clone())
        } else {
            unreachable!("Function type must be Code or VariadicCode")
        };

        // Build the body: call the original function pointer
        let body = func_exp
            .clone()
            .call(parameters.iter().map(|p| p.to_expr()).collect())
            .ret(Location::none());

        // Build and insert the wrapper function itself
        let sym = Symbol::function(
            wrapper_name,
            new_typ,
            Some(Stmt::block(vec![body], Location::none())),
            NO_PRETTY_NAME,
            Location::none(),
        );
        self.symbol_table.insert(sym.clone());
        sym.to_expr().call(args.to_vec())
    }
}

/// Write out information per crate. We need to later aggregate the information
/// for the final combined executable (virtual calls can be across dependencies).
impl VtableCtx {
    /// Write out (1) all call sites and (2) possible concrete methods to JSON.
    pub fn get_virtual_function_restrictions(&mut self) -> VtableCtxResults {
        assert!(self.emit_vtable_restrictions);

        VtableCtxResults {
            call_sites: self.call_sites.clone(),
            possible_methods: self
                .possible_methods
                .drain()
                .map(|(k, v)| PossibleMethodEntry { trait_method: k, possibilities: v })
                .collect(),
        }
    }
}
