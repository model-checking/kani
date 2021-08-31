// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! RMC can be thought of as a translator from an MIR context to a goto context.
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
use crate::gotoc::cbmc::goto_program::{
    DatatypeComponent, Expr, Location, Stmt, Symbol, SymbolTable, Type,
};
use crate::gotoc::cbmc::utils::aggr_name;
use crate::gotoc::cbmc::{MachineModel, RoundingMode};
use crate::gotoc::mir_to_goto::overrides::{type_and_fn_hooks, GotocHooks, GotocTypeHooks};
use crate::gotoc::mir_to_goto::utils::full_crate_name;
use rustc_data_structures::owning_ref::OwningRef;
use rustc_data_structures::rustc_erase_owner;
use rustc_data_structures::stable_map::FxHashMap;
use rustc_data_structures::sync::MetadataRef;
use rustc_middle::middle::cstore::MetadataLoader;
use rustc_middle::mir::interpret::Allocation;
use rustc_middle::ty::layout::{HasParamEnv, HasTyCtxt, TyAndLayout};
use rustc_middle::ty::{self, Instance, Ty, TyCtxt};
use rustc_session::Session;
use rustc_target::abi::Endian;
use rustc_target::abi::{HasDataLayout, LayoutOf, TargetDataLayout};
use rustc_target::spec::Target;
use std::path::Path;

pub struct GotocCtx<'tcx> {
    /// the typing context
    pub tcx: TyCtxt<'tcx>,
    /// the generated symbol table for gotoc
    pub symbol_table: SymbolTable,
    pub hooks: GotocHooks<'tcx>,
    pub type_hooks: GotocTypeHooks<'tcx>,
    /// the full crate name, including versioning info
    pub full_crate_name: String,
    /// a global counter for generating unique names for global variables
    pub global_var_count: u64,
    /// map a global allocation to a name in the symbol table
    pub alloc_map: FxHashMap<&'tcx Allocation, String>,
    pub current_fn: Option<CurrentFnCtx<'tcx>>,
}

/// Constructor
impl<'tcx> GotocCtx<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> GotocCtx<'tcx> {
        let (thks, fhks) = type_and_fn_hooks();
        let mm = machine_model_from_session(tcx.sess);
        let symbol_table = SymbolTable::new(mm);
        GotocCtx {
            tcx,
            symbol_table,
            hooks: fhks,
            type_hooks: thks,
            full_crate_name: full_crate_name(tcx),
            global_var_count: 0,
            alloc_map: FxHashMap::default(),
            current_fn: None,
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
    // Generate a Symbol Expression representing a function variable from the MIR
    pub fn gen_function_local_variable(&mut self, c: u64, fname: &str, t: Type) -> Symbol {
        self.gen_stack_variable(c, fname, "var", t, Location::none())
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
        let base_name = format!("{}_{}", prefix, c);
        let name = format!("{}::1::{}", fname, base_name);
        let symbol = Symbol::variable(name.to_string(), base_name, t, loc);
        self.symbol_table.insert(symbol.clone());
        symbol
    }

    /// Generate a new function local variable that can be used as a temporary in RMC expressions.
    pub fn gen_temp_variable(&mut self, t: Type, loc: Location) -> Symbol {
        let c = self.current_fn_mut().get_and_incr_counter();
        self.gen_stack_variable(c, &self.current_fn().name(), "temp", t, loc)
    }
}

/// Symbol table related
impl<'tcx> GotocCtx<'tcx> {
    /// Ensures that the `name` appears in the Symbol table.
    /// If it doesn't, inserts it using `f`.
    pub fn ensure<F: FnOnce(&mut GotocCtx<'tcx>, &str) -> Symbol>(
        &mut self,
        name: &str,
        f: F,
    ) -> &Symbol {
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
    pub fn ensure_global_var<F: FnOnce(&mut GotocCtx<'tcx>, Expr) -> Option<Stmt>>(
        &mut self,
        name: &str,
        is_file_local: bool,
        t: Type,
        loc: Location,
        init_fn: F,
    ) -> Expr {
        if !self.symbol_table.contains(name) {
            let sym = Symbol::static_variable(name.to_string(), name.to_string(), t.clone(), loc)
                .with_is_file_local(is_file_local);
            let var = sym.to_expr();
            self.symbol_table.insert(sym);
            if let Some(body) = init_fn(self, var) {
                self.register_initializer(name, body);
            }
        }
        self.symbol_table.lookup(name).unwrap().to_expr()
    }

    /// Ensures that a struct with name `struct_name` appears in the symbol table.
    /// If it doesn't, inserts it using `f`.
    /// Returns: a struct-tag referencing the inserted struct.
    pub fn ensure_struct<F: FnOnce(&mut GotocCtx<'tcx>, &str) -> Vec<DatatypeComponent>>(
        &mut self,
        struct_name: &str,
        f: F,
    ) -> Type {
        assert!(!struct_name.starts_with("tag-"));
        if !self.symbol_table.contains(&aggr_name(struct_name)) {
            // Prevent recursion by inserting an incomplete value.
            self.symbol_table.insert(Symbol::incomplete_struct(struct_name));
            let components = f(self, struct_name);
            let sym = Symbol::struct_type(struct_name, components);
            self.symbol_table.replace_with_completion(sym);
        }
        Type::struct_tag(struct_name)
    }

    /// Ensures that a union with name `union_name` appears in the symbol table.
    /// If it doesn't, inserts it using `f`.
    /// Returns: a union-tag referencing the inserted struct.
    pub fn ensure_union<F: FnOnce(&mut GotocCtx<'tcx>, &str) -> Vec<DatatypeComponent>>(
        &mut self,
        union_name: &str,
        f: F,
    ) -> Type {
        assert!(!union_name.starts_with("tag-"));
        if !self.symbol_table.contains(&aggr_name(union_name)) {
            // Prevent recursion by inserting an incomplete value.
            self.symbol_table.insert(Symbol::incomplete_union(union_name));
            let components = f(self, union_name);
            let sym = Symbol::union_type(union_name, components);
            self.symbol_table.replace_with_completion(sym);
        }
        Type::union_tag(union_name)
    }

    pub fn find_function(&mut self, fname: &str) -> Option<Expr> {
        self.symbol_table.lookup(&fname).map(|s| s.to_expr())
    }

    /// Makes a __attribute__((constructor)) fnname() {body} initalizer function
    pub fn register_initializer(&mut self, var_name: &str, body: Stmt) -> &Symbol {
        let fn_name = Self::initializer_fn_name(var_name);
        self.ensure(&fn_name, |_tcx, _| {
            Symbol::function(
                &fn_name,
                Type::code(vec![], Type::constructor()),
                Some(Stmt::block(vec![body], Location::none())), //TODO is this block needed?
                None,
                Location::none(),
            )
            .with_is_file_local(true)
        })
    }
}

/// Mutators
impl<'tcx> GotocCtx<'tcx> {
    pub fn set_current_fn(&mut self, instance: Instance<'tcx>) {
        self.current_fn = Some(CurrentFnCtx::new(instance, self));
    }

    pub fn reset_current_fn(&mut self) {
        self.current_fn = None;
    }

    pub fn next_global_name(&mut self) -> String {
        let c = self.global_var_count;
        self.global_var_count += 1;
        format!("{}::global::{}::", self.full_crate_name(), c)
    }
}

impl LayoutOf<'tcx> for GotocCtx<'tcx> {
    type Ty = Ty<'tcx>;
    type TyAndLayout = TyAndLayout<'tcx>;

    fn layout_of(&self, ty: Self::Ty) -> Self::TyAndLayout {
        self.tcx.layout_of(ty::ParamEnv::reveal_all().and(ty)).unwrap()
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

fn machine_model_from_session(sess: &Session) -> MachineModel {
    // TODO: Hardcoded values from from the ones currently used in env.rs
    // We may wish to get more of them from the session.
    let alignment = sess.target.options.min_global_align.unwrap_or(1);
    let architecture = &sess.target.arch;
    let bool_width = 8;
    let char_is_unsigned = false;
    let char_width = 8;
    let double_width = 64;
    let float_width = 32;
    let int_width = 32;
    let is_big_endian = match sess.target.options.endian {
        Endian::Little => false,
        Endian::Big => true,
    };
    let long_double_width = 128;
    let long_int_width = 64;
    let long_long_int_width = 64;
    let memory_operand_size = 4;
    let null_is_zero = true;
    let pointer_width = sess.target.pointer_width.into();
    let short_int_width = 16;
    let single_width = 32;
    let wchar_t_is_unsigned = false;
    let wchar_t_width = 32;
    let word_size = 32;
    let rounding_mode = RoundingMode::ToNearest;

    MachineModel::new(
        alignment,
        architecture,
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
        memory_operand_size,
        null_is_zero,
        pointer_width,
        rounding_mode,
        short_int_width,
        single_width,
        wchar_t_is_unsigned,
        wchar_t_width,
        word_size,
    )
}
