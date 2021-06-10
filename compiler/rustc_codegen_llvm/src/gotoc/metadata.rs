// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module defines some metadata structures for the codegen

use super::cbmc::goto_program::{
    DatatypeComponent, Expr, Location, Stmt, Symbol, SymbolTable, Type,
};
use super::cbmc::utils::aggr_name;
use crate::gotoc::hooks::{type_and_fn_hooks, GotocHooks, GotocTypeHooks};
use rustc_data_structures::owning_ref::OwningRef;
use rustc_data_structures::rustc_erase_owner;
use rustc_data_structures::stable_map::FxHashMap;
use rustc_data_structures::sync::MetadataRef;
use rustc_hir::def_id::{DefId, LOCAL_CRATE};
use rustc_middle::middle::cstore::MetadataLoader;
use rustc_middle::mir::interpret::Allocation;
use rustc_middle::mir::{BasicBlock, Body, HasLocalDecls, Local, Operand, Place, Rvalue};
use rustc_middle::ty::layout::{HasParamEnv, HasTyCtxt, TyAndLayout};
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::{self, Instance, Ty, TyCtxt, TypeFoldable};
use rustc_target::abi::{HasDataLayout, LayoutOf, TargetDataLayout};
use rustc_target::spec::Target;
use std::iter;
use std::path::Path;
use tracing::debug;

// #[derive(RustcEncodable, RustcDecodable)]
pub struct GotocCodegenResult {
    pub symtab: SymbolTable,
    pub crate_name: rustc_span::Symbol,
}

pub struct GotocMetadataLoader();

pub struct CurrentFnCtx<'tcx> {
    // following fields are updated per function
    /// the codegen instance for the current function
    instance: Instance<'tcx>,
    /// the def id for the current instance
    _def_id: DefId,
    /// the mir for the current instance
    mir: &'tcx Body<'tcx>,
    /// the goto labels for all blocks
    labels: Vec<String>,
    /// the block of the current function body
    pub block: Vec<Stmt>,
    /// the current basic block
    pub current_bb: Option<BasicBlock>,
    /// A counter to enable creating temporary variables
    temp_var_counter: u64,
}

/// Constructor
impl CurrentFnCtx<'tcx> {
    pub fn new(instance: Instance<'tcx>, tcx: TyCtxt<'tcx>) -> Self {
        Self {
            instance,
            mir: tcx.instance_mir(instance.def),
            _def_id: instance.def_id(),
            labels: vec![],
            block: vec![],
            current_bb: None,
            temp_var_counter: 0,
        }
    }
}

/// Setters
impl CurrentFnCtx<'tcx> {
    pub fn get_and_incr_counter(&mut self) -> u64 {
        let rval = self.temp_var_counter;
        self.temp_var_counter += 1;
        rval
    }

    pub fn set_current_bb(&mut self, bb: BasicBlock) {
        self.current_bb = Some(bb);
    }

    pub fn set_labels(&mut self, labels: Vec<String>) {
        assert!(self.labels.is_empty());
        self.labels = labels;
    }
}

/// Getters
impl CurrentFnCtx<'tcx> {
    pub fn labels(&self) -> &Vec<String> {
        &self.labels
    }
}

pub struct GotocCtx<'tcx> {
    /// the typing context
    pub tcx: TyCtxt<'tcx>,
    /// the generated symbol table for gotoc
    pub symbol_table: SymbolTable,
    pub hooks: GotocHooks<'tcx>,
    pub type_hooks: GotocTypeHooks<'tcx>,
    /// a global counter for generating unique names for global variables
    pub global_var_count: u64,
    /// map a global allocation to a name in the symbol table
    pub alloc_map: FxHashMap<&'tcx Allocation, String>,
    pub current_fn: Option<CurrentFnCtx<'tcx>>,
}

impl<'tcx> GotocCtx<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, symbol_table: SymbolTable) -> GotocCtx<'tcx> {
        let (thks, fhks) = type_and_fn_hooks();
        GotocCtx {
            tcx,
            symbol_table,
            hooks: fhks,
            type_hooks: thks,
            global_var_count: 0,
            alloc_map: FxHashMap::default(),
            current_fn: None,
        }
    }

    pub fn instance(&self) -> Instance<'tcx> {
        self.current_fn().instance
    }

    pub fn mir(&self) -> &'tcx Body<'tcx> {
        self.current_fn().mir
    }

    pub fn current_bb(&self) -> BasicBlock {
        self.current_fn().current_bb.unwrap()
    }

    pub fn find_label(&self, bb: &BasicBlock) -> String {
        self.current_fn().labels[bb.index()].clone()
    }

    pub fn set_current_fn(&mut self, instance: Instance<'tcx>) {
        self.current_fn = Some(CurrentFnCtx::new(instance, self.tcx));
    }

    pub fn current_fn(&self) -> &CurrentFnCtx<'tcx> {
        self.current_fn.as_ref().unwrap()
    }

    pub fn current_fn_mut(&mut self) -> &mut CurrentFnCtx<'tcx> {
        self.current_fn.as_mut().unwrap()
    }

    pub fn reset_current_fn(&mut self) {
        self.current_fn = None;
    }

    pub fn crate_name(&self) -> String {
        self.tcx.crate_name(LOCAL_CRATE).to_string()
    }

    #[inline]
    pub fn ptr_width(&self) -> u32 {
        self.tcx.sess.target.pointer_width
    }

    pub fn next_global_name(&mut self) -> String {
        let c = self.global_var_count;
        self.global_var_count += 1;
        format!("{}::global::{}::", self.crate_name(), c)
    }

    /// the name of the current function
    pub fn fname(&self) -> String {
        self.symbol_name(self.instance())
    }

    /// The name of the current function, if there is one.
    /// None, if there is no current function (i.e. we are compiling global state).
    ///
    /// This method returns the function name for contexts describing functions
    /// and the empty string for contexts describing static variables. It
    /// currently returns the managled name. It should return the pretty name.
    pub fn fname_option(&self) -> Option<String> {
        // The function name is contained in the context member named instance,
        // and instance is defined only for function contexts.
        self.current_fn.as_ref().map(|x| self.symbol_name(x.instance))
    }

    /// For the vtable field name, we need exactly the dyn trait name and the function
    /// name. The table itself already is scoped by the object type.
    ///     Example: ::Shape::vol
    /// Note: this is _not_ the same name for top-level entry into the symbol table,
    /// which does need more crate/type information and uses the full symbol_name(...)
    pub fn vtable_field_name(&self, def_id: DefId) -> String {
        // `to_string_no_crate_verbose` is from Rust proper, we use it here because it
        // always includes the dyn trait name and function name.
        // Tracking a less brittle solution here: https://github.com/model-checking/rmc/issues/187
        self.tcx.def_path(def_id).to_string_no_crate_verbose()
    }

    /// A human readable name in Rust for reference, should not be used as a key.
    pub fn readable_instance_name(&self, instance: Instance<'tcx>) -> String {
        with_no_trimmed_paths(|| self.tcx.def_path_str(instance.def_id()))
    }

    /// The actual function name used in the symbol table
    pub fn symbol_name(&self, instance: Instance<'tcx>) -> String {
        let llvm_mangled = self.tcx.symbol_name(instance).name.to_string();
        debug!(
            "finding function name for instance: {}, debug: {:?}, name: {}, symbol: {}, demangle: {}",
            instance,
            instance,
            self.readable_instance_name(instance),
            llvm_mangled,
            rustc_demangle::demangle(&llvm_mangled).to_string()
        );

        let pretty = self.readable_instance_name(instance);

        // Make main function a special case for easy CBMC entry
        // TODO: probably need to edit for https://github.com/model-checking/rmc/issues/169
        if pretty == "main" {
            "main".to_string()
        } else {
            // TODO: llvm mangled string is not very readable. one way to tackle this is to
            // demangle it. but the demangled string has no generic info.
            // the best scenario is to use v0 mangler, but this is not default at this moment.
            // this is the kind of tiny but annoying issue.
            // c.f. https://github.com/rust-lang/rust/issues/60705
            //
            // the following solution won't work pretty:
            // match self.tcx.sess.opts.debugging_opts.symbol_mangling_version {
            //     SymbolManglingVersion::Legacy => llvm_mangled,
            //     SymbolManglingVersion::V0 => rustc_demangle::demangle(llvm_mangled.as_str()).to_string(),
            // }
            llvm_mangled
        }
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
            let sym = Symbol::variable(name.to_string(), name.to_string(), t.clone(), loc)
                .with_is_file_local(is_file_local)
                .with_is_thread_local(false)
                .with_is_static_lifetime(true);
            let var = sym.to_expr();
            self.symbol_table.insert(sym);
            if let Some(body) = init_fn(self, var) {
                self.register_initializer(name, body);
            }
        }
        self.symbol_table.lookup(name).unwrap().to_expr()
    }

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

    pub fn monomorphize<T>(&self, value: T) -> T
    where
        T: TypeFoldable<'tcx>,
    {
        // Instance is Some(..) only when current codegen unit is a function.
        if self.current_fn.is_some() {
            self.instance().subst_mir_and_normalize_erasing_regions(
                self.tcx,
                ty::ParamEnv::reveal_all(),
                value,
            )
        } else {
            // TODO: confirm with rust team there is no way to monomorphize
            // a global value.
            value
        }
    }

    pub fn local_ty(&self, l: Local) -> Ty<'tcx> {
        self.monomorphize(self.mir().local_decls()[l].ty)
    }

    pub fn rvalue_ty(&self, rv: &Rvalue<'tcx>) -> Ty<'tcx> {
        self.monomorphize(rv.ty(self.mir().local_decls(), self.tcx))
    }

    pub fn operand_ty(&self, o: &Operand<'tcx>) -> Ty<'tcx> {
        self.monomorphize(o.ty(self.mir().local_decls(), self.tcx))
    }

    pub fn place_ty(&self, p: &Place<'tcx>) -> Ty<'tcx> {
        self.monomorphize(p.ty(self.mir().local_decls(), self.tcx).ty)
    }

    pub fn closure_params(&self, substs: ty::subst::SubstsRef<'tcx>) -> Vec<Ty<'tcx>> {
        let sig = self.monomorphize(substs.as_closure().sig());
        let args = match sig.skip_binder().inputs()[0].kind() {
            ty::Tuple(substs) => substs.iter().map(|s| s.expect_ty()),
            _ => unreachable!("this argument of a closure must be a tuple"),
        };
        args.collect()
    }

    fn closure_sig(
        &self,
        def_id: DefId,
        substs: ty::subst::SubstsRef<'tcx>,
    ) -> ty::PolyFnSig<'tcx> {
        let sig = self.monomorphize(substs.as_closure().sig());

        // In addition to `def_id` and `substs`, we need to provide the kind of region `env_region`
        // in `closure_env_ty`, which we can build from the bound variables as follows
        let bound_vars = self.tcx.mk_bound_variable_kinds(
            sig.bound_vars().iter().chain(iter::once(ty::BoundVariableKind::Region(ty::BrEnv))),
        );
        let br = ty::BoundRegion {
            var: ty::BoundVar::from_usize(bound_vars.len() - 1),
            kind: ty::BoundRegionKind::BrEnv,
        };
        let env_region = ty::ReLateBound(ty::INNERMOST, br);
        let env_ty = self.tcx.closure_env_ty(def_id, substs, env_region).unwrap();

        // The parameter types are tupled, but we want to have them in a vector
        let params = self.closure_params(substs);
        let sig = sig.skip_binder();

        // We build a binder from `sig` where:
        //  * `inputs` contains a sequence with the closure and parameter types
        //  * the rest of attributes are obtained from `sig`
        ty::Binder::bind_with_vars(
            self.tcx.mk_fn_sig(
                iter::once(env_ty).chain(params.iter().cloned()),
                sig.output(),
                sig.c_variadic,
                sig.unsafety,
                sig.abi,
            ),
            bound_vars,
        )
    }

    pub fn fn_sig_of_instance(&self, instance: Instance<'tcx>) -> ty::PolyFnSig<'tcx> {
        let fntyp = instance.ty(self.tcx, ty::ParamEnv::reveal_all());
        self.monomorphize(match fntyp.kind() {
            ty::Closure(def_id, subst) => self.closure_sig(*def_id, subst),
            _ => fntyp.fn_sig(self.tcx),
        })
    }

    pub fn fn_sig(&self) -> ty::PolyFnSig<'tcx> {
        self.fn_sig_of_instance(self.instance())
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

    // Generate a Symbol Expression representing a function variable from the MIR
    pub fn gen_function_local_variable(&mut self, c: u64, fname: &str, t: Type) -> Symbol {
        self.gen_stack_variable(c, fname, "var", t, Location::none())
    }

    /// Generate a new function local variable that can be used as a temporary in RMC expressions.
    pub fn gen_temp_variable(&mut self, t: Type, loc: Location) -> Symbol {
        let c = self.current_fn_mut().get_and_incr_counter();
        self.gen_stack_variable(c, &self.fname(), "temp", t, loc)
    }

    pub fn find_function(&mut self, fname: &str) -> Option<Expr> {
        self.symbol_table.lookup(&fname).map(|s| s.to_expr())
    }

    pub fn initializer_fn_name(var_name: &str) -> String {
        format!("{}_init", var_name)
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

    /// RMC does not currently support all MIR constructs.
    /// When we hit a construct we don't handle, we have two choices:
    /// We can use the `unimplemented!()` macro, which causes a compile time failure.
    /// Or, we can use this function, which inserts an `assert(false, "FOO is not currently supported by RMC")` into the generated code.
    /// This means that if the unimplemented feature is dynamically used by the code being verified, we will see an assertion failure.
    /// If it is not used, we the assertion will pass.
    /// This allows us to continue to make progress parsing rust code, while remaining sound (thanks to the `assert(false)`)
    ///
    /// TODO: https://github.com/model-checking/rmc/issues/8 assume the required validity constraints for the nondet return
    /// TODO: https://github.com/model-checking/rmc/issues/9 Have a parameter that decides whether to `assume(0)` to block further traces or not
    pub fn codegen_unimplemented(
        &mut self,
        operation_name: &str,
        t: Type,
        loc: Location,
        url: &str,
    ) -> Expr {
        let body = vec![
            // Assert false to alert the user that there is a path that uses an unimplemented feature.
            Stmt::assert_false(
                &format!(
                    "{} is not currently supported by RMC. Please post your example at {} ",
                    operation_name, url
                ),
                loc.clone(),
            ),
            // Assume false to block any further exploration of this path.
            Stmt::assume(Expr::bool_false(), loc.clone()),
            t.nondet().as_stmt(loc.clone()).with_location(loc.clone()), //TODO assume rust validity contraints
        ];

        Expr::statement_expression(body, t).with_location(loc)
    }
}

impl<'tcx> LayoutOf for GotocCtx<'tcx> {
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
