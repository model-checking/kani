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
//! Any MIR specific functionality (e.g. codegen etc) should live in specialized files that use
//! this structure as input.
use super::current_fn::CurrentFnCtx;
use super::vtable_ctx::VtableCtx;
use crate::codegen_cprover_gotoc::overrides::{fn_hooks, GotocHooks};
use crate::codegen_cprover_gotoc::utils::full_crate_name;
use cbmc::goto_program::{DatatypeComponent, Expr, Location, Stmt, Symbol, SymbolTable, Type};
use cbmc::utils::aggr_tag;
use cbmc::InternedString;
use cbmc::{MachineModel, RoundingMode};
use kani_metadata::{HarnessMetadata, UnsupportedFeature};
use kani_queries::{QueryDb, UserInput};
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::owning_ref::OwningRef;
use rustc_data_structures::rustc_erase_owner;
use rustc_data_structures::sync::MetadataRef;
use rustc_middle::mir::interpret::Allocation;
use rustc_middle::span_bug;
use rustc_middle::ty::layout::{
    FnAbiError, FnAbiOfHelpers, FnAbiRequest, HasParamEnv, HasTyCtxt, LayoutError, LayoutOfHelpers,
    TyAndLayout,
};
use rustc_middle::ty::{self, Instance, Ty, TyCtxt};
use rustc_session::cstore::MetadataLoader;
use rustc_session::Session;
use rustc_span::source_map::{respan, Span};
use rustc_target::abi::call::FnAbi;
use rustc_target::abi::Endian;
use rustc_target::abi::{HasDataLayout, TargetDataLayout};
use rustc_target::spec::Target;
use std::path::Path;

pub struct GotocCtx<'tcx> {
    /// the typing context
    pub tcx: TyCtxt<'tcx>,
    /// a snapshot of the query values. The queries shouldn't change at this point,
    /// so we just keep a copy.
    pub queries: QueryDb,
    /// the generated symbol table for gotoc
    pub symbol_table: SymbolTable,
    pub hooks: GotocHooks<'tcx>,
    /// the full crate name, including versioning info
    pub full_crate_name: String,
    /// a global counter for generating unique names for global variables
    pub global_var_count: u64,
    /// map a global allocation to a name in the symbol table
    pub alloc_map: FxHashMap<&'tcx Allocation, String>,
    /// map (trait, method) pairs to possible implementations
    pub vtable_ctx: VtableCtx,
    pub current_fn: Option<CurrentFnCtx<'tcx>>,
    pub type_map: FxHashMap<InternedString, Ty<'tcx>>,
    /// map from symbol identifier to string literal
    /// TODO: consider making the map from Expr to String instead
    pub str_literals: FxHashMap<InternedString, String>,
    pub proof_harnesses: Vec<HarnessMetadata>,
    pub test_harnesses: Vec<HarnessMetadata>,
    /// a global counter for generating unique IDs for checks
    pub global_checks_count: u64,
    /// A map of unsupported constructs that were found while codegen
    pub unsupported_constructs: FxHashMap<InternedString, Vec<Location>>,
    /// A map of concurrency constructs that are treated sequentially.
    /// We collect them and print one warning at the end if not empty instead of printing one
    /// warning at each occurrence.
    pub concurrent_constructs: FxHashMap<InternedString, Vec<Location>>,
}

/// Constructor
impl<'tcx> GotocCtx<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, queries: QueryDb) -> GotocCtx<'tcx> {
        let fhks = fn_hooks();
        let mm = machine_model_from_session(tcx.sess);
        let symbol_table = SymbolTable::new(mm);
        let emit_vtable_restrictions = queries.get_emit_vtable_restrictions();
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
            proof_harnesses: vec![],
            test_harnesses: vec![],
            global_checks_count: 0,
            unsupported_constructs: FxHashMap::default(),
            concurrent_constructs: FxHashMap::default(),
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

    /// Maps the goto-context "unsupported features" data into the
    /// KaniMetadata "unsupported features" format.
    ///
    /// These are different because the KaniMetadata is a flat serializable list,
    /// while we need a more richly structured HashMap in the goto context.
    pub(crate) fn unsupported_metadata(&self) -> Vec<UnsupportedFeature> {
        self.unsupported_constructs
            .iter()
            .map(|(construct, location)| UnsupportedFeature {
                feature: construct.to_string(),
                locations: location
                    .iter()
                    .map(|l| {
                        // We likely (and should) have no instances of
                        // calling `codegen_unimplemented` without file/line.
                        // So while we map out of `Option` here, we expect them to always be `Some`
                        kani_metadata::Location {
                            filename: l.filename().unwrap_or_default(),
                            start_line: l.start_line().unwrap_or_default(),
                        }
                    })
                    .collect(),
            })
            .collect()
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
    pub fn gen_function_local_variable(&mut self, c: u64, fname: &str, t: Type) -> Symbol {
        self.gen_stack_variable(c, fname, "var", t, Location::none(), false)
    }

    // Generate a Symbol Expression representing a function parameter from the MIR
    pub fn gen_function_parameter(&mut self, c: u64, fname: &str, t: Type) -> Symbol {
        self.gen_stack_variable(c, fname, "var", t, Location::none(), true)
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
        is_param: bool,
    ) -> Symbol {
        let base_name = format!("{prefix}_{c}");
        let name = format!("{fname}::1::{base_name}");
        let symbol = Symbol::variable(name, base_name, t, loc).with_is_parameter(is_param);
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
        let var =
            self.gen_stack_variable(c, &self.current_fn().name(), "temp", t, loc, false).to_expr();
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

    /// Ensures that a global variable `name` appears in the Symbol table.
    /// If it doesn't, inserts it.
    /// If `init_fn` returns `Some(body)`, creates an initializer for the variable using `body`.
    /// Otherwise, leaves the variable uninitialized .
    pub fn ensure_global_var<
        F: FnOnce(&mut GotocCtx<'tcx>, Expr) -> Option<Stmt>,
        T: Into<InternedString>,
    >(
        &mut self,
        name: T,
        is_file_local: bool,
        t: Type,
        loc: Location,
        init_fn: F,
    ) -> Expr {
        let name = name.into();
        if !self.symbol_table.contains(name) {
            tracing::debug!(?name, "Ensure global variable");
            let sym = Symbol::static_variable(name, name, t, loc)
                .with_is_file_local(is_file_local)
                .with_is_hidden(false);
            let var = sym.to_expr();
            self.symbol_table.insert(sym);
            if let Some(body) = init_fn(self, var) {
                self.register_initializer(&name.to_string(), body);
            }
        }
        self.symbol_table.lookup(name).unwrap().to_expr()
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
    pub fn register_initializer(&mut self, var_name: &str, body: Stmt) -> &Symbol {
        let fn_name = Self::initializer_fn_name(var_name);
        let pretty_name = format!("{var_name}::init");
        self.ensure(&fn_name, |_tcx, _| {
            Symbol::function(
                &fn_name,
                Type::code(vec![], Type::constructor()),
                Some(Stmt::block(vec![body], Location::none())), //TODO is this block needed?
                &pretty_name,
                Location::none(),
            )
            .with_is_file_local(true)
        })
    }
}

/// Mutators
impl<'tcx> GotocCtx<'tcx> {
    pub fn set_current_fn(&mut self, instance: Instance<'tcx>) {
        self.current_fn = Some(CurrentFnCtx::new(
            instance,
            self,
            self.tcx
                .instance_mir(instance.def)
                .basic_blocks
                .indices()
                .map(|bb| format!("{bb:?}"))
                .collect(),
        ));
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
            self.tcx.sess.emit_fatal(respan(span, err))
        } else {
            match fn_abi_request {
                FnAbiRequest::OfFnPtr { sig, extra_args } => {
                    span_bug!(
                        span,
                        "Error: {err}\n while running `fn_abi_of_fn_ptr. ({sig}, {extra_args:?})`",
                    );
                }
                FnAbiRequest::OfInstance { instance, extra_args } => {
                    span_bug!(
                        span,
                        "Error: {err}\n while running `fn_abi_of_instance. ({instance}, {extra_args:?})`",
                    );
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct GotocMetadataLoader();
impl MetadataLoader for GotocMetadataLoader {
    fn get_rlib_metadata(&self, _: &Target, _filename: &Path) -> Result<MetadataRef, String> {
        let buf = vec![];
        let buf: OwningRef<Vec<u8>, [u8]> = OwningRef::new(buf);
        Ok(rustc_erase_owner!(buf.map_owner_box()))
    }

    fn get_dylib_metadata(&self, target: &Target, filename: &Path) -> Result<MetadataRef, String> {
        self.get_rlib_metadata(target, filename)
    }
}

/// Builds a machine model which is required by CBMC
fn machine_model_from_session(sess: &Session) -> MachineModel {
    // The model assumes a `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`
    // or `aarch64-apple-darwin` platform. We check the target platform in function
    // `check_target` from src/kani-compiler/src/codegen_cprover_gotoc/compiler_interface.rs
    // and error if it is not any of the ones we expect.
    let architecture = &sess.target.arch;
    let pointer_width = sess.target.pointer_width.into();

    // The model assumes the following values for session options:
    //   * `min_global_align`: 1
    //   * `endian`: `Endian::Little`
    //
    // We check these options in function `check_options` from
    // src/kani-compiler/src/codegen_cprover_gotoc/compiler_interface.rs
    // and error if their values are not the ones we expect.
    let alignment = sess.target.options.min_global_align.unwrap_or(1);
    let is_big_endian = match sess.target.options.endian {
        Endian::Little => false,
        Endian::Big => true,
    };

    // The values below cannot be obtained from the session so they are
    // hardcoded using standard ones for the supported platforms
    // see /tools/sizeofs/main.cpp.
    // For reference, the definition in CBMC:
    //https://github.com/diffblue/cbmc/blob/develop/src/util/config.cpp
    match architecture.as_ref() {
        "x86_64" => {
            let bool_width = 8;
            let char_is_unsigned = false;
            let char_width = 8;
            let double_width = 64;
            let float_width = 32;
            let int_width = 32;
            let long_double_width = 128;
            let long_int_width = 64;
            let long_long_int_width = 64;
            let short_int_width = 16;
            let single_width = 32;
            let wchar_t_is_unsigned = false;
            let wchar_t_width = 32;

            MachineModel {
                architecture: architecture.to_string(),
                alignment,
                bool_width,
                char_is_unsigned,
                char_width,
                double_width,
                float_width,
                int_width,
                is_big_endian,
                long_double_width,
                long_int_width,
                long_long_int_width,
                memory_operand_size: int_width / 8,
                null_is_zero: true,
                pointer_width,
                rounding_mode: RoundingMode::ToNearest,
                short_int_width,
                single_width,
                wchar_t_is_unsigned,
                wchar_t_width,
                word_size: int_width,
            }
        }
        "aarch64" => {
            let bool_width = 8;
            let char_is_unsigned = true;
            let char_width = 8;
            let double_width = 64;
            let float_width = 32;
            let int_width = 32;
            let long_double_width = 64;
            let long_int_width = 64;
            let long_long_int_width = 64;
            let short_int_width = 16;
            let single_width = 32;
            let wchar_t_is_unsigned = false;
            let wchar_t_width = 32;

            MachineModel {
                // CBMC calls it arm64, not aarch64
                architecture: "arm64".to_string(),
                alignment,
                bool_width,
                char_is_unsigned,
                char_width,
                double_width,
                float_width,
                int_width,
                is_big_endian,
                long_double_width,
                long_int_width,
                long_long_int_width,
                memory_operand_size: int_width / 8,
                null_is_zero: true,
                pointer_width,
                rounding_mode: RoundingMode::ToNearest,
                short_int_width,
                single_width,
                wchar_t_is_unsigned,
                wchar_t_width,
                word_size: int_width,
            }
        }
        _ => {
            panic!("Unsupported architecture: {architecture}");
        }
    }
}
