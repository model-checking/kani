// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module defines some metadata structures for the codegen

use super::cbmc::goto_program::{
    DatatypeComponent, Expr, Location, Stmt, Symbol, SymbolTable, Type,
};
use super::cbmc::utils::aggr_name;
use crate::gotoc::current_fn::CurrentFnCtx;
use crate::gotoc::hooks::{type_and_fn_hooks, GotocHooks, GotocTypeHooks};
use rustc_data_structures::owning_ref::OwningRef;
use rustc_data_structures::rustc_erase_owner;
use rustc_data_structures::stable_map::FxHashMap;
use rustc_data_structures::sync::MetadataRef;
use rustc_hir::def_id::{DefId, LOCAL_CRATE};
use rustc_middle::middle::cstore::MetadataLoader;
use rustc_middle::mir::interpret::Allocation;
use rustc_middle::mir::{HasLocalDecls, Local, Operand, Place, Rvalue};
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

    pub fn set_current_fn(&mut self, instance: Instance<'tcx>) {
        self.current_fn = Some(CurrentFnCtx::new(instance, self));
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

    /// The name for the struct field on a vtable for a given function. Because generic
    /// functions can share the same name, we need to use the index of the entry in the
    /// vtable. This is the same index that will be passed in virtual function calls as
    /// InstanceDef::Virtual(def_id, idx). We could use solely the index as a key into
    /// the vtable struct, but we add the method name for debugging readability.
    ///     Example: 3_vol
    pub fn vtable_field_name(&self, _def_id: DefId, idx: usize) -> String {
        // format!("{}_{}", idx, with_no_trimmed_paths(|| self.tcx.item_name(def_id)))
        // TODO: use def_id https://github.com/model-checking/rmc/issues/364
        idx.to_string()
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
        if let Some(current_fn) = &self.current_fn {
            current_fn.instance().subst_mir_and_normalize_erasing_regions(
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
        self.monomorphize(self.current_fn().mir().local_decls()[l].ty)
    }

    pub fn rvalue_ty(&self, rv: &Rvalue<'tcx>) -> Ty<'tcx> {
        self.monomorphize(rv.ty(self.current_fn().mir().local_decls(), self.tcx))
    }

    pub fn operand_ty(&self, o: &Operand<'tcx>) -> Ty<'tcx> {
        self.monomorphize(o.ty(self.current_fn().mir().local_decls(), self.tcx))
    }

    pub fn place_ty(&self, p: &Place<'tcx>) -> Ty<'tcx> {
        self.monomorphize(p.ty(self.current_fn().mir().local_decls(), self.tcx).ty)
    }

    /// Closures expect their last arg untupled at call site, see comment at
    /// ty_needs_closure_untupled.
    fn sig_with_closure_untupled(&self, sig: ty::PolyFnSig<'tcx>) -> ty::PolyFnSig<'tcx> {
        debug!("sig_with_closure_untupled sig: {:?}", sig);
        let fn_sig = sig.skip_binder();
        if let Some((tupe, prev_args)) = fn_sig.inputs().split_last() {
            let args: Vec<Ty<'tcx>> = match tupe.kind() {
                ty::Tuple(substs) => substs.iter().map(|s| s.expect_ty()),
                _ => unreachable!("the final argument of a closure must be a tuple"),
            }
            .collect();

            // The leading argument should be exactly the environment
            assert!(prev_args.len() == 1);
            let env = prev_args[0].clone();

            // Recombine arguments: environment first, then the flattened tuple elements
            let recombined_args = iter::once(env).chain(args);

            return ty::Binder::bind_with_vars(
                self.tcx.mk_fn_sig(
                    recombined_args,
                    fn_sig.output(),
                    fn_sig.c_variadic,
                    fn_sig.unsafety,
                    fn_sig.abi,
                ),
                sig.bound_vars(),
            );
        }
        sig
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

        let sig = sig.skip_binder();

        // We build a binder from `sig` where:
        //  * `inputs` contains a sequence with the closure and parameter types
        //  * the rest of attributes are obtained from `sig`
        let sig = ty::Binder::bind_with_vars(
            self.tcx.mk_fn_sig(
                iter::once(env_ty).chain(iter::once(sig.inputs()[0])),
                sig.output(),
                sig.c_variadic,
                sig.unsafety,
                sig.abi,
            ),
            bound_vars,
        );

        // The parameter types are tupled, but we want to have them in a vector
        self.sig_with_closure_untupled(sig)
    }

    pub fn fn_sig_of_instance(&self, instance: Instance<'tcx>) -> ty::PolyFnSig<'tcx> {
        let fntyp = instance.ty(self.tcx, ty::ParamEnv::reveal_all());
        self.monomorphize(match fntyp.kind() {
            ty::Closure(def_id, subst) => self.closure_sig(*def_id, subst),
            ty::FnPtr(..) | ty::FnDef(..) => {
                let sig = fntyp.fn_sig(self.tcx);
                // Some virtual calls through a vtable may actually be closures
                // or shims that also need the arguments untupled, even though
                // the kind of the trait type is not a ty::Closure.
                if self.ty_needs_closure_untupled(fntyp) {
                    return self.sig_with_closure_untupled(sig);
                }
                sig
            }
            _ => unreachable!("Can't get function signature of type: {:?}", fntyp),
        })
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
        self.gen_stack_variable(c, &self.current_fn().name(), "temp", t, loc)
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
