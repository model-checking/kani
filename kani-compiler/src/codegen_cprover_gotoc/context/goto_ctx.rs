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
use crate::codegen_cprover_gotoc::overrides::{fn_hooks, GotocHooks};
use crate::codegen_cprover_gotoc::utils::full_crate_name;
use crate::codegen_cprover_gotoc::UnsupportedConstructs;
use crate::kani_middle::transform::BodyTransformation;
use crate::kani_queries::QueryDb;
use cbmc::goto_program::{DatatypeComponent, Expr, Location, Stmt, Symbol, SymbolTable, Type};
use cbmc::utils::aggr_tag;
use cbmc::{InternedString, MachineModel};
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::span_bug;
use rustc_middle::ty::layout::{
    FnAbiError, FnAbiOfHelpers, FnAbiRequest, HasParamEnv, HasTyCtxt, LayoutError, LayoutOfHelpers,
    TyAndLayout,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::source_map::respan;
use rustc_span::Span;
use rustc_target::abi::call::FnAbi;
use rustc_target::abi::{HasDataLayout, TargetDataLayout};
use stable_mir::mir::mono::Instance;
use stable_mir::mir::Body;
use stable_mir::ty::Allocation;

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
impl<'tcx> GotocCtx<'tcx> {
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

    // Generate a Symbol Expression representing a function variable from the MIR
    pub fn gen_function_local_variable(
        &mut self,
        c: u64,
        fname: &str,
        t: Type,
        loc: Location,
    ) -> Symbol {
        self.gen_stack_variable(c, fname, "var", t, loc)
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
    pub fn ensure_global_var_init<
        F: FnOnce(&mut GotocCtx<'tcx>, Expr) -> Stmt,
        T: Into<InternedString> + Clone,
    >(
        &mut self,
        name: T,
        is_file_local: bool,
        is_const: bool,
        t: Type,
        loc: Location,
        init_fn: F,
    ) -> Expr {
        let sym = self.ensure_global_var(name, is_file_local, t, loc);
        self.register_initializer(&sym, init_fn);
        sym.to_expr()
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
    ) -> Symbol {
        let sym_name = name.clone().into();
        if let Some(sym) = self.symbol_table.lookup(sym_name) {
            sym.clone()
        } else {
            tracing::debug!(?sym_name, "ensure_global_var insert");
            let sym = Symbol::static_variable(sym_name, sym_name, t, loc)
                .with_is_file_local(is_file_local)
                .with_is_hidden(false);
            self.symbol_table.insert(sym.clone());
            sym
        }
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

    /// Makes a `__attribute__((constructor)) fnname() {body}` initalizer function
    pub fn register_initializer<F>(&mut self, var: &Symbol, init_fn: F) -> &Symbol
    where
        F: FnOnce(&mut GotocCtx<'tcx>, Expr) -> Stmt,
    {
        let var_name = var.name.to_string();
        let fn_name = Self::initializer_fn_name(&var_name);
        let pretty_name = format!("{var_name}::init");
        self.ensure(&fn_name, |gcx, _| {
            let body = init_fn(gcx, var.to_expr());
            let loc = *body.location();
            Symbol::function(
                &fn_name,
                Type::code(vec![], Type::constructor()),
                Some(Stmt::block(vec![body], loc)), //TODO is this block needed?
                &pretty_name,
                loc,
            )
            .with_is_file_local(true)
        })
    }
}

/// Mutators
impl<'tcx> GotocCtx<'tcx> {
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

impl<'tcx> HasParamEnv<'tcx> for GotocCtx<'tcx> {
    fn param_env(&self) -> ty::ParamEnv<'tcx> {
        ty::ParamEnv::reveal_all()
    }
}

impl<'tcx> HasTyCtxt<'tcx> for GotocCtx<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }
}

impl<'tcx> HasDataLayout for GotocCtx<'tcx> {
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
