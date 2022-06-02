// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{DatatypeComponent, Expr, Location, Parameter, Symbol, SymbolTable, Type};
use cbmc::utils::aggr_tag;
use cbmc::{btree_map, NO_PRETTY_NAME};
use cbmc::{InternString, InternedString};
use rustc_ast::ast::Mutability;
use rustc_index::vec::IndexVec;
use rustc_middle::mir::{HasLocalDecls, Local, Operand, Place, Rvalue};
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::print::FmtPrinter;
use rustc_middle::ty::subst::InternalSubsts;
use rustc_middle::ty::{
    self, AdtDef, FloatTy, Instance, IntTy, PolyFnSig, Ty, UintTy, VariantDef, VtblEntry,
};
use rustc_middle::ty::{List, TypeFoldable};
use rustc_span;
use rustc_span::def_id::DefId;
use rustc_target::abi::{
    Abi::Vector, FieldsShape, Integer, Layout, Primitive, TagEncoding, VariantIdx, Variants,
};
use rustc_target::spec::abi::Abi;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt::Debug;
use std::iter;
use std::iter::FromIterator;
use tracing::{debug, trace, warn};
use ty::layout::HasParamEnv;

/// Map the unit type to an empty struct
///
/// Mapping unit to `void` works for functions with no return type but not for variables with type
/// unit. We treat both uniformly by declaring an empty struct type: `struct Unit {}` and a global
/// variable `struct Unit VoidUnit` returned by all void functions.
const UNIT_TYPE_EMPTY_STRUCT_NAME: &str = "Unit";
pub const FN_RETURN_VOID_VAR_NAME: &str = "VoidUnit";

/// Map the never i.e. `!` type to an empty struct.
/// The never type can appear as a function argument, e.g. in library/core/src/num/error.rs
const NEVER_TYPE_EMPTY_STRUCT_NAME: &str = "Never";

pub trait TypeExt {
    fn is_rust_fat_ptr(&self, st: &SymbolTable) -> bool;
    fn is_rust_slice_fat_ptr(&self, st: &SymbolTable) -> bool;
    fn is_rust_trait_fat_ptr(&self, st: &SymbolTable) -> bool;
    fn is_unit(&self) -> bool;
    fn is_unit_pointer(&self) -> bool;
    fn unit() -> Self;
}

impl TypeExt for Type {
    fn is_rust_slice_fat_ptr(&self, st: &SymbolTable) -> bool {
        match self {
            Type::Struct { components, .. } => {
                components.len() == 2
                    && components.iter().any(|x| x.name() == "data" && x.typ().is_pointer())
                    && components.iter().any(|x| x.name() == "len" && x.typ().is_integer())
            }
            Type::StructTag(tag) => st.lookup(*tag).unwrap().typ.is_rust_slice_fat_ptr(st),
            _ => false,
        }
    }

    fn is_rust_trait_fat_ptr(&self, st: &SymbolTable) -> bool {
        match self {
            Type::Struct { components, .. } => {
                components.len() == 2
                    && components.iter().any(|x| x.name() == "data" && x.typ().is_pointer())
                    && components.iter().any(|x| x.name() == "vtable" && x.typ().is_pointer())
            }
            Type::StructTag(tag) => {
                st.lookup(&tag.to_string()).unwrap().typ.is_rust_trait_fat_ptr(st)
            }
            _ => false,
        }
    }

    fn is_rust_fat_ptr(&self, st: &SymbolTable) -> bool {
        self.is_rust_slice_fat_ptr(st) || self.is_rust_trait_fat_ptr(st)
    }

    fn unit() -> Self {
        // We depend on GotocCtx::codegen_ty_unit() to put the type in the symbol table.
        // We don't have access to the symbol table here to do it ourselves.
        Type::struct_tag(UNIT_TYPE_EMPTY_STRUCT_NAME)
    }

    fn is_unit(&self) -> bool {
        match self {
            Type::StructTag(name) => *name == aggr_tag(UNIT_TYPE_EMPTY_STRUCT_NAME),
            _ => false,
        }
    }

    fn is_unit_pointer(&self) -> bool {
        match self {
            Type::Pointer { typ } => typ.is_unit(),
            _ => false,
        }
    }
}
trait ExprExt {
    fn unit(symbol_table: &SymbolTable) -> Self;

    fn is_unit(&self) -> bool;

    fn is_unit_pointer(&self) -> bool;
}

impl ExprExt for Expr {
    fn unit(symbol_table: &SymbolTable) -> Self {
        Expr::struct_expr(Type::unit(), btree_map![], symbol_table)
    }

    fn is_unit(&self) -> bool {
        self.typ().is_unit()
    }

    fn is_unit_pointer(&self) -> bool {
        self.typ().is_unit_pointer()
    }
}

/// Function signatures
impl<'tcx> GotocCtx<'tcx> {
    /// Closures expect their last arg untupled at call site, see comment at
    /// ty_needs_closure_untupled.
    fn sig_with_closure_untupled(&self, sig: ty::PolyFnSig<'tcx>) -> ty::PolyFnSig<'tcx> {
        debug!("sig_with_closure_untupled sig: {:?}", sig);
        let fn_sig = sig.skip_binder();
        if let Some((tupe, prev_args)) = fn_sig.inputs().split_last() {
            let args = match *tupe.kind() {
                ty::Tuple(substs) => substs,
                _ => unreachable!("the final argument of a closure must be a tuple"),
            };

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

    pub fn fn_sig_of_instance(&self, instance: Instance<'tcx>) -> Option<ty::PolyFnSig<'tcx>> {
        let fntyp = instance.ty(self.tcx, ty::ParamEnv::reveal_all());
        self.monomorphize(match fntyp.kind() {
            ty::Closure(def_id, subst) => Some(self.closure_sig(*def_id, subst)),
            ty::FnPtr(..) | ty::FnDef(..) => {
                let sig = fntyp.fn_sig(self.tcx);
                // Some virtual calls through a vtable may actually be closures
                // or shims that also need the arguments untupled, even though
                // the kind of the trait type is not a ty::Closure.
                if self.ty_needs_closure_untupled(fntyp) {
                    return Some(self.sig_with_closure_untupled(sig));
                }
                Some(sig)
            }
            ty::Generator(_def_id, _substs, _movability) => {
                let error_msg = GotocCtx::unsupported_msg(
                    "The `generators` feature",
                    Some("https://github.com/model-checking/kani/issues/416"),
                );
                self.emit_error_and_exit(&error_msg)
            }
            _ => unreachable!("Can't get function signature of type: {:?}", fntyp),
        })
    }
}

impl<'tcx> GotocCtx<'tcx> {
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

    /// Is the MIR type an unsized type (i.e. one represented by a fat pointer?)
    pub fn is_unsized(&self, t: Ty<'tcx>) -> bool {
        !self
            .monomorphize(t)
            .is_sized(self.tcx.at(rustc_span::DUMMY_SP), ty::ParamEnv::reveal_all())
    }

    /// Is the MIR type a ref of an unsized type (i.e. one represented by a fat pointer?)
    pub fn is_ref_of_unsized(&self, t: Ty<'tcx>) -> bool {
        match t.kind() {
            ty::Ref(_, to, _) | ty::RawPtr(ty::TypeAndMut { ty: to, .. }) => self.is_unsized(*to),
            _ => false,
        }
    }

    /// Is the MIR type a ref of an unsized type (i.e. one represented by a fat pointer?)
    pub fn is_ref_of_sized(&self, t: Ty<'tcx>) -> bool {
        match t.kind() {
            ty::Ref(_, to, _) | ty::RawPtr(ty::TypeAndMut { ty: to, .. }) => !self.is_unsized(*to),
            _ => false,
        }
    }

    /// Is the MIR type a box of an unsized type (i.e. one represented by a fat pointer?)
    pub fn is_box_of_unsized(&self, t: Ty<'tcx>) -> bool {
        if t.is_box() {
            let boxed_t = self.monomorphize(t.boxed_ty());
            self.is_unsized(boxed_t)
        } else {
            false
        }
    }

    /// Generates the type for a single field for a dynamic vtable.
    /// In particular, these fields are function pointers.
    fn trait_method_vtable_field_type(
        &mut self,
        instance: Instance<'tcx>,
        idx: usize,
    ) -> DatatypeComponent {
        // Gives a binder with function signature
        let sig = self.fn_sig_of_instance(instance).unwrap();

        // Gives an Irep Pointer object for the signature
        let fn_ty = self.codegen_dynamic_function_sig(sig);
        let fn_ptr = fn_ty.to_pointer();

        // vtable field name, i.e., 3_vol (idx_method)
        let vtable_field_name = self.vtable_field_name(instance.def_id(), idx);

        DatatypeComponent::field(vtable_field_name, fn_ptr)
    }

    /// Generates a vtable that looks like this:
    ///   struct io::error::vtable {
    ///      void *drop_in_place;
    ///      size_t size;
    ///      size_t align;
    ///      int (*f)(int) f1;
    ///      ...
    ///   }
    /// Ensures that the vtable is added to the symbol table.
    fn codegen_trait_vtable_type(&mut self, t: ty::Ty<'tcx>) -> Type {
        self.ensure_struct(self.vtable_name(t), NO_PRETTY_NAME, |ctx, _| {
            ctx.trait_vtable_field_types(t)
        })
    }

    /// This will codegen the trait data type. Since this is unsized, we just create a typedef.
    ///
    /// This is relevant when generating the layout of unsized types like `RcBox`.
    /// ```
    /// struct RcBox<T: ?Sized> {
    ///     strong: Cell<usize>,
    ///     weak: Cell<usize>,
    ///     value: T,
    /// }
    /// ```
    ///
    /// This behaviour is similar to slices, and `value` is not a pointer.
    /// `value` is the concrete object in memory which was casted to an unsized type.
    pub fn codegen_trait_data(&mut self, t: ty::Ty<'tcx>) -> Type {
        let name = self.normalized_trait_name(t);
        let inner_name = name.clone() + "Inner";
        debug!(typ=?t, kind=?t.kind(), %name, %inner_name,
                "codegen_trait_data_type");
        self.ensure(inner_name.clone(), |_ctx, _| {
            Symbol::typedef(
                &inner_name,
                &inner_name,
                Type::unit().to_typedef(inner_name.clone()),
                Location::None,
            )
        });
        Type::unit().to_typedef(inner_name)
    }

    /// This will codegen the raw pointer to the trait data.
    pub fn codegen_trait_data_pointer(&mut self, typ: ty::Ty<'tcx>) -> Type {
        assert!(typ.is_trait());
        self.codegen_ty(typ).to_pointer()
    }

    /// A reference to a Struct<dyn T> { .., data: T} is translated to
    /// struct RefToTrait {
    ///     Struct<dyn T>* data;
    ///     Metadata<dyn T>* vtable;
    /// }
    /// Note: T is a `typedef` but data represents the space in memory occupied by
    /// the concrete type. We just don't know its size during compilation time.
    fn codegen_trait_fat_ptr_type(
        &mut self,
        pointee_type: ty::Ty<'tcx>,
        trait_type: ty::Ty<'tcx>,
    ) -> Type {
        trace!(?pointee_type, ?trait_type, "codegen_trait_fat_ptr_type");
        let name = self.ty_mangled_name(pointee_type).to_string() + "::FatPtr";
        let data_type = self.codegen_ty(pointee_type).to_pointer();
        self.ensure_struct(&name, NO_PRETTY_NAME, |ctx, _| {
            // At this point in time, the vtable hasn't been codegen yet.
            // However, all we need to know is its name, which we do know.
            // See the comment on codegen_ty_ref.
            let vtable_name = ctx.vtable_name(trait_type);
            vec![
                DatatypeComponent::field("data", data_type),
                DatatypeComponent::field("vtable", Type::struct_tag(vtable_name).to_pointer()),
            ]
        })
    }

    /// `drop_in_place` is a function with type &self -> (), the vtable for
    /// dynamic trait objects needs a pointer to it
    pub fn trait_vtable_drop_type(&mut self, t: ty::Ty<'tcx>) -> Type {
        if matches!(t.kind(), ty::Dynamic(..)) {
            Type::code_with_unnamed_parameters(
                vec![self.codegen_fat_ptr(t).to_pointer()],
                Type::unit(),
            )
            .to_pointer()
        } else {
            Type::code_with_unnamed_parameters(vec![self.codegen_ty(t).to_pointer()], Type::unit())
                .to_pointer()
        }
    }

    /// Given a trait of type `t`, determine the fields of the struct that will implement its vtable.
    ///
    /// The order of fields (i.e., the layout of a vtable) is not guaranteed by the compiler.
    /// We follow the order implemented by the compiler in compiler/rustc_codegen_ssa/src/meth.rs
    /// `get_vtable`.
    fn trait_vtable_field_types(&mut self, t: ty::Ty<'tcx>) -> Vec<DatatypeComponent> {
        let mut vtable_base = vec![
            DatatypeComponent::field("drop", self.trait_vtable_drop_type(t)),
            DatatypeComponent::field("size", Type::size_t()),
            DatatypeComponent::field("align", Type::size_t()),
        ];
        if let ty::Dynamic(binder, _region) = t.kind() {
            // The virtual methods on the trait ref. Some auto traits have no methods.
            if let Some(principal) = binder.principal() {
                let poly = principal.with_self_ty(self.tcx, t);
                let poly = self.tcx.erase_regions(poly);
                let mut flds = self
                    .tcx
                    .vtable_entries(poly)
                    .iter()
                    .cloned()
                    .enumerate()
                    .filter_map(|(idx, entry)| match entry {
                        VtblEntry::Method(instance) => {
                            Some(self.trait_method_vtable_field_type(instance, idx))
                        }
                        // TODO: trait upcasting
                        // https://github.com/model-checking/kani/issues/358
                        VtblEntry::TraitVPtr(..) => None,
                        VtblEntry::MetadataDropInPlace
                        | VtblEntry::MetadataSize
                        | VtblEntry::MetadataAlign
                        | VtblEntry::Vacant => None,
                    })
                    .collect();

                vtable_base.append(&mut flds);
            }
            vtable_base
        } else {
            unreachable!("Expected to get a dynamic object here");
        }
    }

    /// Gives the name for a trait, i.e., `dyn T`. This does not work for `&dyn T`.
    pub fn normalized_trait_name(&self, t: Ty<'tcx>) -> String {
        assert!(t.is_trait(), "Type {} must be a trait type (a dynamic type)", t);
        self.ty_mangled_name(t).to_string()
    }

    /// Gives the vtable name for a type.
    /// In some cases, we have &T, in other cases T, so normalize.
    ///
    /// TODO: to handle trait upcasting, this will need to use a
    /// poly existential trait type as a part of the key as well.
    /// See compiler/rustc_middle/src/ty/vtable.rs
    /// https://github.com/model-checking/kani/issues/358
    pub fn vtable_name(&self, t: Ty<'tcx>) -> String {
        format!("{}::vtable", self.normalized_trait_name(t))
    }

    pub fn ty_pretty_name(&self, t: Ty<'tcx>) -> InternedString {
        use rustc_hir::def::Namespace;
        use rustc_middle::ty::print::Printer;
        let printer = FmtPrinter::new(self.tcx, Namespace::TypeNS);

        // Monomorphizing the type ensures we get a cannonical form for dynamic trait
        // objects with auto traits, such as:
        //   StructTag("tag-std::boxed::Box<(dyn std::error::Error + std::marker::Send + std::marker::Sync)>") }
        //   StructTag("tag-std::boxed::Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>") }
        let t = self.monomorphize(t);
        with_no_trimmed_paths!(printer.print_type(t).unwrap().into_buffer()).intern()
    }

    pub fn ty_mangled_name(&self, t: Ty<'tcx>) -> InternedString {
        // Crate resolution: mangled names need to be distinct across different versions
        // of the same crate that could be pulled in by dependencies. However, Kani's
        // treatment of FFI C calls asssumes that we generate the same name for structs
        // as the C name, so don't mangle in that case.
        // TODO: this is likely insufficient if a dependent crate has two versions of
        // linked C libraries
        // https://github.com/model-checking/kani/issues/450
        if is_repr_c_adt(t) {
            self.ty_pretty_name(t)
        } else {
            // This hash is documented to be the same no matter the crate context
            let id_u64 = self.tcx.type_id_hash(t);
            format!("_{}", id_u64).intern()
        }
    }

    #[allow(dead_code)]
    pub fn enum_union_name(&self, ty: Ty<'tcx>) -> String {
        format!("{}-union", self.ty_mangled_name(ty))
    }

    #[allow(dead_code)]
    pub fn enum_case_struct_name(&self, ty: Ty<'tcx>, case: &VariantDef) -> String {
        format!("{}::{}", self.ty_mangled_name(ty), case.name)
    }

    pub fn codegen_ty_raw_array(&mut self, ty: Ty<'tcx>) -> Type {
        match ty.kind() {
            ty::Array(t, c) => {
                let size = self.codegen_const(*c, None).int_constant_value().unwrap();
                let elemt = self.codegen_ty(*t);
                elemt.array_of(size)
            }
            _ => unreachable!("should only call on array"),
        }
    }

    /// A foreign type is a type that rust does not know the contents of.
    /// We handle this by treating it as an incomplete struct.
    fn codegen_foreign(&mut self, ty: Ty<'tcx>, defid: DefId) -> Type {
        debug!("codegen_foreign {:?} {:?}", ty, defid);
        let name = self.ty_mangled_name(ty).intern();
        self.ensure(aggr_tag(name), |ctx, _| {
            Symbol::incomplete_struct(name, Some(ctx.ty_pretty_name(ty)))
        });
        Type::struct_tag(name)
    }

    /// The unit type in Rust is an empty struct in gotoc
    pub fn codegen_ty_unit(&mut self) -> Type {
        self.ensure_struct(UNIT_TYPE_EMPTY_STRUCT_NAME, NO_PRETTY_NAME, |_, _| vec![])
    }

    /// codegen for types. it finds a C type which corresponds to a rust type.
    /// that means [ty] has to be monomorphized.
    ///
    /// check [LayoutCx::layout_raw_uncached] for LLVM codegen
    ///
    /// also c.f. https://www.ralfj.de/blog/2020/04/04/layout-debugging.html
    ///      c.f. https://rust-lang.github.io/unsafe-code-guidelines/introduction.html
    pub fn codegen_ty(&mut self, ty: Ty<'tcx>) -> Type {
        let normalized = self.tcx.normalize_erasing_regions(ty::ParamEnv::reveal_all(), ty);
        let goto_typ = self.codegen_ty_inner(normalized);
        if let Some(tag) = goto_typ.tag() {
            if !self.type_map.contains_key(&tag) {
                debug!(mir_type=?normalized, gotoc_name=?tag, ?goto_typ,  "codegen_ty: new type");
                self.type_map.insert(tag, normalized);
            }
        }
        goto_typ
    }

    fn codegen_ty_inner(&mut self, ty: Ty<'tcx>) -> Type {
        trace!(typ=?ty, "codegen_ty");
        match ty.kind() {
            ty::Int(k) => self.codegen_iint(*k),
            ty::Bool => Type::c_bool(),
            ty::Char => Type::signed_int(32),
            ty::Uint(k) => self.codegen_uint(*k),
            ty::Float(k) => match k {
                FloatTy::F32 => Type::float(),
                FloatTy::F64 => Type::double(),
            },
            ty::Adt(def, _) if def.repr().simd() => self.codegen_vector(ty),
            ty::Adt(def, subst) => {
                debug!("variants are: {:?}", def.variants());
                if def.is_struct() {
                    self.codegen_struct(ty, def, subst)
                } else if def.is_union() {
                    self.codegen_union(ty, def, subst)
                } else {
                    self.codegen_enum(ty, def, subst)
                }
            }
            ty::Foreign(defid) => self.codegen_foreign(ty, *defid),
            ty::Array(et, len) => {
                let array_name = format!(
                    "[{}; {}]",
                    self.ty_mangled_name(*et),
                    len.try_eval_usize(self.tcx, self.param_env()).unwrap()
                );
                // wrap arrays into struct so that one can take advantage of struct copy in C
                //
                // struct [T; n] {
                //   T _0[n];
                // }
                self.ensure_struct(&array_name, NO_PRETTY_NAME, |ctx, _| {
                    if et.is_unit() {
                        // we do not generate a struct with an array of units
                        vec![]
                    } else {
                        vec![DatatypeComponent::field(
                            &0usize.to_string(),
                            ctx.codegen_ty_raw_array(ty),
                        )]
                    }
                })
            }
            ty::Dynamic(..) => {
                // This is `dyn Trait` not a reference.
                self.codegen_trait_data(ty)
            }
            // As per zulip, a raw slice/str is a variable length array
            // https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/Memory.20layout.20of.20DST
            // &[T] -> { data: *const T, len: usize }
            // [T] -> memory location (flexible array)
            // Note: This is not valid C but CBMC seems to be ok with it.
            ty::Slice(e) => self.codegen_ty(*e).flexible_array_of(),
            ty::Str => Type::c_char().flexible_array_of(),
            ty::Ref(_, t, _) | ty::RawPtr(ty::TypeAndMut { ty: t, .. }) => self.codegen_ty_ref(*t),
            ty::FnDef(_, _) => {
                let sig = self.monomorphize(ty.fn_sig(self.tcx));
                self.codegen_function_sig(sig)
            }
            ty::FnPtr(sig) => self.codegen_function_sig(*sig).to_pointer(),
            ty::Closure(_, subst) => self.codegen_ty_closure(ty, subst),
            ty::Generator(_, subst, _) => self.codegen_ty_generator(subst),
            ty::Never => {
                self.ensure_struct(NEVER_TYPE_EMPTY_STRUCT_NAME, NO_PRETTY_NAME, |_, _| vec![])
            }
            ty::Tuple(ts) => {
                if ts.is_empty() {
                    self.codegen_ty_unit()
                } else {
                    // we do not have to do two insertions for tuple because it is impossible for
                    // finite tuples to loop.
                    self.ensure_struct(
                        self.ty_mangled_name(ty),
                        Some(self.ty_pretty_name(ty)),
                        |tcx, _| tcx.codegen_ty_tuple_fields(ty, *ts),
                    )
                }
            }
            ty::Projection(_) | ty::Opaque(_, _) => {
                unreachable!("Type should've been normalized already")
            }

            // shouldn't come to here after mormomorphization
            ty::Bound(_, _) | ty::Param(_) => unreachable!("monomorphization bug"),

            // type checking remnants which shouldn't be reachable
            ty::GeneratorWitness(_) | ty::Infer(_) | ty::Placeholder(_) | ty::Error(_) => {
                unreachable!("remnants of type checking")
            }
        }
    }

    pub(crate) fn codegen_iint(&self, k: IntTy) -> Type {
        match k {
            IntTy::I8 => Type::signed_int(8),
            IntTy::I16 => Type::signed_int(16),
            IntTy::I32 => Type::signed_int(32),
            IntTy::I64 => Type::signed_int(64),
            IntTy::I128 => Type::signed_int(128),
            IntTy::Isize => Type::ssize_t(),
        }
    }

    pub fn codegen_uint(&self, k: UintTy) -> Type {
        match k {
            UintTy::U8 => Type::unsigned_int(8),
            UintTy::U16 => Type::unsigned_int(16),
            UintTy::U32 => Type::unsigned_int(32),
            UintTy::U64 => Type::unsigned_int(64),
            UintTy::U128 => Type::unsigned_int(128),
            UintTy::Usize => Type::size_t(),
        }
    }

    fn codegen_ty_tuple_fields(
        &mut self,
        t: Ty<'tcx>,
        tys: &List<Ty<'tcx>>,
    ) -> Vec<DatatypeComponent> {
        self.codegen_ty_tuple_like(t, tys.to_vec())
    }

    fn codegen_struct_padding<T>(
        &self,
        current_offset: T,
        next_offset: T,
        idx: usize,
    ) -> Option<DatatypeComponent>
    where
        T: TryInto<u64>,
        T::Error: Debug,
    {
        let current_offset: u64 = current_offset.try_into().unwrap();
        let next_offset: u64 = next_offset.try_into().unwrap();
        assert!(current_offset <= next_offset);
        if current_offset < next_offset {
            // We need to pad to the next offset
            let bits = next_offset - current_offset;
            let name = format!("$pad{}", idx);
            Some(DatatypeComponent::padding(&name, bits))
        } else {
            None
        }
    }

    /// generate a struct based on the layout
    /// the fields and types are determined by flds while their order is determined by layout.
    ///
    /// once the order is determined, this function also computes padding fields based on the size
    /// and the offset of each field as appropriate.
    ///
    /// * name - the name of the struct
    /// * flds - list of field name and type pairs, but the order is not specified by this list
    /// * layout - layout of the struct
    /// * initial_offset - offset which has been accumulated in parent struct, in bits
    fn codegen_struct_fields(
        &mut self,
        flds: Vec<(String, Ty<'tcx>)>,
        layout: &Layout,
        initial_offset: usize,
    ) -> Vec<DatatypeComponent> {
        match &layout.fields() {
            FieldsShape::Arbitrary { offsets, memory_index } => {
                assert_eq!(flds.len(), offsets.len());
                assert_eq!(offsets.len(), memory_index.len());
                let mut final_fields = Vec::with_capacity(flds.len());
                let mut offset: u64 = initial_offset.try_into().unwrap();
                for idx in layout.fields().index_by_increasing_offset() {
                    let fld_offset = offsets[idx].bits();
                    let (fld_name, fld_ty) = &flds[idx];
                    if let Some(padding) =
                        self.codegen_struct_padding(offset, fld_offset, final_fields.len())
                    {
                        final_fields.push(padding)
                    }
                    // we insert the actual field
                    final_fields.push(DatatypeComponent::field(fld_name, self.codegen_ty(*fld_ty)));
                    let layout = self.layout_of(*fld_ty);
                    // we compute the overall offset of the end of the current struct
                    offset = fld_offset + layout.size.bits();
                }

                // If we don't meet our expected alignment, pad until we do
                let align = layout.align().abi.bits();
                let overhang = offset % align;
                if overhang != 0 {
                    final_fields.push(
                        self.codegen_struct_padding(
                            offset,
                            offset + align - overhang,
                            final_fields.len(),
                        )
                        .unwrap(),
                    )
                }

                final_fields
            }
            // Primitives, such as NEVER, have no fields
            FieldsShape::Primitive => vec![],
            _ => unreachable!("{}\n{:?}", self.current_fn().readable_name(), layout.fields()),
        }
    }

    fn codegen_ty_tuple_like(&mut self, t: Ty<'tcx>, tys: Vec<Ty<'tcx>>) -> Vec<DatatypeComponent> {
        let layout = self.layout_of(t);
        let flds: Vec<_> =
            tys.iter().enumerate().map(|(i, t)| (GotocCtx::tuple_fld_name(i), *t)).collect();
        // tuple cannot have other initial offset
        self.codegen_struct_fields(flds, &layout.layout, 0)
    }

    /// A closure in Rust MIR takes two arguments:
    ///    0. a struct representing the environment
    ///    1. a tuple containing the parameters
    ///
    /// However, during codegen/lowering from MIR, the 2nd tuple of parameters
    /// is flattened into subsequent parameters.
    ///
    /// Checking whether the type's kind is a closure is insufficient, because
    /// a virtual method call through a vtable can have the trait's non-closure
    /// type. For example:
    ///         let p: &dyn Fn(i32) = &|x| assert!(x == 1);
    ///         p(1);
    ///
    /// Here, the call p(1) desugars to an MIR trait call Fn::call(&p, (1,)),
    /// where the second argument is a tuple. The instance type kind for
    /// Fn::call is not a closure, because dynamically, the pointer may be to
    /// a function definition instead. We still need to untuple in this case,
    /// so we follow the example elsewhere in Rust to use the ABI call type.
    /// See `make_call_args` in kani/compiler/rustc_mir/src/transform/inline.rs
    pub fn ty_needs_closure_untupled(&self, ty: Ty<'tcx>) -> bool {
        match ty.kind() {
            ty::FnDef(..) | ty::FnPtr(..) => ty.fn_sig(self.tcx).abi() == Abi::RustCall,
            _ => unreachable!("Can't treat type as a function: {:?}", ty),
        }
    }

    /// A closure is a struct of all its environments. That is, a closure is
    /// just a tuple with a unique type identifier, so that Fn related traits
    /// can find its impl.
    fn codegen_ty_closure(&mut self, t: Ty<'tcx>, substs: ty::subst::SubstsRef<'tcx>) -> Type {
        self.ensure_struct(self.ty_mangled_name(t), Some(self.ty_pretty_name(t)), |ctx, _| {
            ctx.codegen_ty_tuple_like(t, substs.as_closure().upvar_tys().collect())
        })
    }

    /// Preliminary support for the Generator type kind. The core functionality remains
    /// unimplemented, but this way we fail at verification time only if paths that
    /// rely on Generator types are used.
    fn codegen_ty_generator(&mut self, substs: ty::subst::SubstsRef<'tcx>) -> Type {
        let tys = substs.as_generator().upvar_tys().map(|t| self.codegen_ty(t)).collect();
        let output = self.codegen_ty(substs.as_generator().return_ty());
        Type::code_with_unnamed_parameters(tys, output)
    }

    /// Codegen "fat pointers" to the given `pointee_type`. These are pointers with metadata.
    ///
    /// There are three kinds of fat pointers:
    /// 1. references to slices (`matches!(pointee_type.kind(), ty::Slice(..) | ty::Str)`).
    /// 2. references to trait objects (`matches!(pointee_type.kind(), ty::Dynamic)`).
    /// 3. references to structs whose last field is a unsized object (slice / trait)
    ///    - `matches!(pointee_type.kind(), ty::Adt(..) if self.is_unsized(t))
    ///
    pub fn codegen_fat_ptr(&mut self, pointee_type: Ty<'tcx>) -> Type {
        assert!(
            !self.use_thin_pointer(pointee_type),
            "Generating a fat pointer for a type requiring a thin pointer: {:?}",
            pointee_type.kind()
        );
        if self.use_slice_fat_pointer(pointee_type) {
            let pointer_name = match pointee_type.kind() {
                ty::Slice(..) => self.ty_mangled_name(pointee_type),
                ty::Str => "refstr".intern(),
                ty::Adt(..) => format!("&{}", self.ty_mangled_name(pointee_type)).intern(),
                kind => unreachable!("Generating a slice fat pointer to {:?}", kind),
            };
            let element_type = match pointee_type.kind() {
                ty::Slice(elt_type) => self.codegen_ty(*elt_type),
                ty::Str => Type::c_char(),
                // For adt, see https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp
                ty::Adt(..) => self.codegen_ty(pointee_type),
                kind => unreachable!("Generating a slice fat pointer to {:?}", kind),
            };
            self.ensure_struct(pointer_name, NO_PRETTY_NAME, |_, _| {
                vec![
                    DatatypeComponent::field("data", element_type.to_pointer()),
                    DatatypeComponent::field("len", Type::size_t()),
                ]
            })
        } else if self.use_vtable_fat_pointer(pointee_type) {
            // Pointee type can either be `dyn T` or `Struct<dyn T>`.
            // The vtable for both cases is the vtable of `dyn T`.
            let trait_type = self.extract_trait_type(pointee_type).unwrap();
            self.codegen_trait_vtable_type(trait_type);
            self.codegen_trait_fat_ptr_type(pointee_type, trait_type)
        } else {
            unreachable!(
                "A pointer is either a thin pointer, slice fat pointer, or vtable fat pointer."
            );
        }
    }

    pub fn codegen_ty_ref(&mut self, pointee_type: Ty<'tcx>) -> Type {
        // Normalize pointee_type to remove projection and opaque types
        trace!(?pointee_type, "codegen_ty_ref");
        let pointee_type =
            self.tcx.normalize_erasing_regions(ty::ParamEnv::reveal_all(), pointee_type);

        if !self.use_thin_pointer(pointee_type) {
            return self.codegen_fat_ptr(pointee_type);
        }

        match pointee_type.kind() {
            ty::Dynamic(..) | ty::Slice(_) | ty::Str => {
                unreachable!("Should have generated a fat pointer")
            }
            ty::Projection(_) | ty::Opaque(..) => {
                unreachable!("Should have been removed by normalization")
            }

            // We have a "thin pointer", which is just a pointer
            ty::Adt(..)
            | ty::Array(..)
            | ty::Bool
            | ty::Char
            | ty::Closure(..)
            | ty::Float(_)
            | ty::Foreign(_)
            | ty::Int(_)
            | ty::RawPtr(_)
            | ty::Ref(..)
            | ty::Tuple(_)
            | ty::Uint(_) => self.codegen_ty(pointee_type).to_pointer(),

            // These types were blocking firecracker. Doing the default thing to unblock.
            // https://github.com/model-checking/kani/issues/215
            // https://github.com/model-checking/kani/issues/216
            ty::FnDef(_, _) | ty::Never => self.codegen_ty(pointee_type).to_pointer(),

            // These types were blocking stdlib. Doing the default thing to unblock.
            // https://github.com/model-checking/kani/issues/214
            ty::FnPtr(_) => self.codegen_ty(pointee_type).to_pointer(),

            // Use the default for a reference to a generator. This is a
            // temporary workaround to allow codegen to continue to a point
            // where it either:
            // 1. codegens unimplemented for the generator OR
            // 2. errors out
            // Adding full support for generators is tracked by:
            // https://github.com/model-checking/kani/issues/416
            ty::Generator(_, _, _) => self.codegen_ty(pointee_type).to_pointer(),

            // These types have no regression tests for them.
            // For soundness, hold off on generating them till we have test-cases.
            ty::Bound(_, _) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::Error(_) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::GeneratorWitness(_) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::Infer(_) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::Param(_) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::Placeholder(_) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
        }
    }

    /// Generate code for a trait function declaration.
    ///
    /// Dynamic function calls first parameter is the fat-pointer representing self.
    /// For closures, the type of the first argument is dyn T not &dyn T.
    pub fn codegen_dynamic_function_sig(&mut self, sig: PolyFnSig<'tcx>) -> Type {
        let sig = self.monomorphize(sig);
        let sig = self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig);

        let mut is_first = true;
        let params = sig
            .inputs()
            .iter()
            .filter_map(|arg_type| {
                if is_first {
                    // This should &dyn T or dyn T (for closures).
                    is_first = false;
                    let first_ty = pointee_type(*arg_type).unwrap_or(*arg_type);
                    debug!(self_type=?arg_type, fn_signature=?sig, "codegen_dynamic_function_sig");
                    assert!(first_ty.is_trait(), "Expected self type to be a trait");
                    Some(self.codegen_trait_data_pointer(first_ty))
                } else if self.ignore_var_ty(*arg_type) {
                    debug!("Ignoring type {:?} in function signature", arg_type);
                    None
                } else {
                    debug!("Using type {:?} in function signature", arg_type);
                    Some(self.codegen_ty(*arg_type))
                }
            })
            .collect();

        Type::code_with_unnamed_parameters(params, self.codegen_ty(sig.output()))
    }

    /// one can only apply this function to a monomorphized signature
    pub fn codegen_function_sig(&mut self, sig: PolyFnSig<'tcx>) -> Type {
        let sig = self.monomorphize(sig);
        let sig = self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig);
        let params = sig
            .inputs()
            .iter()
            .filter_map(|t| {
                if self.ignore_var_ty(*t) {
                    debug!("Ignoring type {:?} in function signature", t);
                    None
                } else {
                    debug!("Using type {:?} in function signature", t);
                    Some(self.codegen_ty(*t))
                }
            })
            .collect();

        if sig.c_variadic {
            Type::variadic_code_with_unnamed_parameters(params, self.codegen_ty(sig.output()))
        } else {
            Type::code_with_unnamed_parameters(params, self.codegen_ty(sig.output()))
        }
    }

    /// codegen for struct
    ///
    /// they are literally codegen'ed in the corresponding way (except the order of fields might not be preserved)
    fn codegen_struct(
        &mut self,
        ty: Ty<'tcx>,
        def: &'tcx AdtDef,
        subst: &'tcx InternalSubsts<'tcx>,
    ) -> Type {
        self.ensure_struct(self.ty_mangled_name(ty), Some(self.ty_pretty_name(ty)), |ctx, _| {
            let variant = &def.variants().raw[0];
            let layout = ctx.layout_of(ty);
            ctx.codegen_variant_struct_fields(variant, subst, &layout.layout, 0)
        })
    }

    /// generate a struct representing the layout of the variant
    fn codegen_variant_struct_fields(
        &mut self,
        variant: &VariantDef,
        subst: &'tcx InternalSubsts<'tcx>,
        layout: &Layout,
        initial_offset: usize,
    ) -> Vec<DatatypeComponent> {
        let flds: Vec<_> =
            variant.fields.iter().map(|f| (f.name.to_string(), f.ty(self.tcx, subst))).collect();
        self.codegen_struct_fields(flds, layout, initial_offset)
    }

    /// codegen unions
    fn codegen_union(
        &mut self,
        ty: Ty<'tcx>,
        def: &'tcx AdtDef,
        subst: &'tcx InternalSubsts<'tcx>,
    ) -> Type {
        self.ensure_union(self.ty_mangled_name(ty), Some(self.ty_pretty_name(ty)), |ctx, _| {
            def.variants().raw[0]
                .fields
                .iter()
                .map(|f| {
                    DatatypeComponent::field(
                        &f.name.to_string(),
                        ctx.codegen_ty(f.ty(ctx.tcx, subst)),
                    )
                })
                .collect()
        })
    }

    /// Mapping enums to CBMC types is rather complicated. There are a few cases to consider:
    /// 1. When there is only 0 or 1 variant, this is straightforward as the code shows
    /// 2. When there are more variants, rust might decides to apply the typical encoding which
    /// regard enums as tagged union, or an optimized form, called niche encoding.
    ///
    /// The direct encoding is straightforward. Enums are just mapped to C as a struct of union of structs.
    /// e.g.
    ///     enum Foo {
    ///       A(T1, T2),
    ///       B(T3, T4),
    ///     }
    /// is translated to
    /// struct Foo {
    ///   isize case, // discriminant
    ///   union {
    ///     struct Foo::A {
    ///       T1 _0; T2 _1;
    ///     } A;
    ///     struct Foo::B {
    ///       T3 _0; T4 _1;
    ///     } B;
    ///   } cases;
    /// }
    ///
    /// The niche encoding is an optimization and a complication. This optimization occurs, when
    /// Rust knows certain data does not have certain values. For example, a reference is not null.
    /// In that case, the Location::none() case in Option<&'a i32> gets mapped to the null value, and the whole
    /// type is just the same as &i32. This type is translated to the following type:
    /// struct Option<&i32> {
    ///     u8 *_0;
    /// }
    /// c.f. https://rust-lang.github.io/unsafe-code-guidelines/layout/enums.html#layout-of-a-data-carrying-enums-without-a-repr-annotation
    fn codegen_enum(
        &mut self,
        ty: Ty<'tcx>,
        adtdef: &'tcx AdtDef,
        subst: &'tcx InternalSubsts<'tcx>,
    ) -> Type {
        self.ensure_struct(self.ty_mangled_name(ty), Some(self.ty_pretty_name(ty)), |ctx, name| {
            // variants appearing in source code (in source code order)
            let source_variants = &adtdef.variants();
            // variants appearing in mir code
            match &ctx.layout_of(ty).variants {
                Variants::Single { index } => {
                    match source_variants.get(*index) {
                        None => {
                            // an empty enum with no variants (its value cannot be instantiated)
                            vec![]
                        }
                        Some(variant) => {
                            // a single enum is pretty much like a struct
                            let layout = ctx.layout_of(ty).layout;
                            ctx.codegen_variant_struct_fields(variant, subst, &layout, 0)
                        }
                    }
                }
                Variants::Multiple { tag_encoding, variants, .. } => {
                    match tag_encoding {
                        TagEncoding::Direct => {
                            // For direct encoding of tags, we generate a type with two fields:
                            // ```
                            // struct tag-<> { // enum type
                            //    case: <discriminant  type>,
                            //    cases: tag-<>-union,
                            // }
                            // ```
                            // The `case` field type determined by the enum representation
                            // (`#[repr]`) and it represents which variant is being used.
                            // The `cases` field is a union of all variant types where the name
                            // of each union field is the name of the corresponding discriminant.
                            let discr_t = ctx.codegen_enum_discr_typ(ty);
                            let int = ctx.codegen_ty(discr_t);
                            let discr_offset = ctx.layout_of(discr_t).size.bits_usize();
                            let initial_offset =
                                ctx.variant_min_offset(variants).unwrap_or(discr_offset);
                            let mut fields = vec![DatatypeComponent::field("case", int)];
                            if let Some(padding) =
                                ctx.codegen_struct_padding(discr_offset, initial_offset, 0)
                            {
                                fields.push(padding);
                            }
                            fields.push(DatatypeComponent::field(
                                "cases",
                                ctx.ensure_union(
                                    &format!("{}-union", name.to_string()),
                                    NO_PRETTY_NAME,
                                    |ctx, name| {
                                        ctx.codegen_enum_cases(
                                            name,
                                            adtdef,
                                            subst,
                                            variants,
                                            initial_offset,
                                        )
                                    },
                                ),
                            ));
                            fields
                        }
                        TagEncoding::Niche { dataful_variant, .. } => {
                            // Enumerations with multiple variants and niche encoding have a
                            // specific format that can be used to optimize its layout and reduce
                            // memory consumption.
                            //
                            // These enumerations have one and only one variant with non-ZST
                            // fields which is referred to by the `dataful_variant` index. Their
                            // final size and alignment is equal to the one from the
                            // `dataful_variant`. All other variants either don't have any field
                            // or all fields types are ZST.
                            //
                            // Because of that, we can represent these enums as simple structures
                            // where each field represent one variant. This allows them to be
                            // referred to correctly.
                            //
                            // Note: I tried using a union instead but it had a significant runtime
                            // penalty.
                            tracing::trace!(
                                ?name,
                                ?variants,
                                ?dataful_variant,
                                ?tag_encoding,
                                ?subst,
                                "codegen_enum: Niche"
                            );
                            ctx.codegen_enum_cases(name, adtdef, subst, variants, 0)
                        }
                    }
                }
            }
        })
    }

    pub(crate) fn variant_min_offset(
        &self,
        variants: &IndexVec<VariantIdx, Layout>,
    ) -> Option<usize> {
        variants
            .iter()
            .filter_map(|lo| {
                if lo.fields().count() == 0 {
                    None
                } else {
                    // get the offset of the leftmost field, which is the one
                    // with the least offset since we codegen fields in a struct
                    // in the order of increasing offsets. Note that this is not
                    // necessarily the 0th field since the compiler may reorder
                    // fields.
                    Some(
                        lo.fields()
                            .offset(lo.fields().index_by_increasing_offset().nth(0).unwrap())
                            .bits_usize(),
                    )
                }
            })
            .min()
    }

    pub fn codegen_prim_typ(&self, primitive: Primitive) -> Ty<'tcx> {
        match primitive {
            Primitive::Int(k, signed) => match k {
                Integer::I8 => {
                    if signed {
                        self.tcx.types.i8
                    } else {
                        self.tcx.types.u8
                    }
                }
                Integer::I16 => {
                    if signed {
                        self.tcx.types.i16
                    } else {
                        self.tcx.types.u16
                    }
                }
                Integer::I32 => {
                    if signed {
                        self.tcx.types.i32
                    } else {
                        self.tcx.types.u32
                    }
                }
                Integer::I64 => {
                    if signed {
                        self.tcx.types.i64
                    } else {
                        self.tcx.types.u64
                    }
                }
                Integer::I128 => {
                    if signed {
                        self.tcx.types.i128
                    } else {
                        self.tcx.types.u128
                    }
                }
            },

            Primitive::F32 => self.tcx.types.f32,
            Primitive::F64 => self.tcx.types.f64,
            Primitive::Pointer => {
                self.tcx.mk_ptr(ty::TypeAndMut { ty: self.tcx.types.u8, mutbl: Mutability::Not })
            }
        }
    }

    pub fn codegen_enum_discr_typ(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        let layout = self.layout_of(ty);
        match &layout.variants {
            Variants::Multiple { tag, .. } => self.codegen_prim_typ(tag.primitive()),
            _ => unreachable!("only enum has discriminant"),
        }
    }

    /// Codegen the type for each variant represented in this enum.
    /// As an optimization, we ignore the ones that don't have any field, since they
    /// are only manipulated via discriminant operations.
    fn codegen_enum_cases(
        &mut self,
        name: InternedString,
        def: &'tcx AdtDef,
        subst: &'tcx InternalSubsts<'tcx>,
        layouts: &IndexVec<VariantIdx, Layout>,
        initial_offset: usize,
    ) -> Vec<DatatypeComponent> {
        def.variants()
            .iter_enumerated()
            .filter_map(|(i, case)| {
                if case.fields.is_empty() {
                    // Skip variant types that cannot be referenced.
                    None
                } else {
                    Some(DatatypeComponent::field(
                        &case.name.to_string(),
                        self.codegen_enum_case_struct(
                            name,
                            case,
                            subst,
                            &layouts[i],
                            initial_offset,
                        ),
                    ))
                }
            })
            .collect()
    }

    fn codegen_enum_case_struct(
        &mut self,
        name: InternedString,
        case: &VariantDef,
        subst: &'tcx InternalSubsts<'tcx>,
        variant: &Layout,
        initial_offset: usize,
    ) -> Type {
        let case_name = format!("{}::{}", name.to_string(), case.name);
        debug!("handling variant {}: {:?}", case_name, case);
        self.ensure_struct(&case_name, NO_PRETTY_NAME, |tcx, _| {
            tcx.codegen_variant_struct_fields(case, subst, variant, initial_offset)
        })
    }

    fn codegen_vector(&mut self, ty: Ty<'tcx>) -> Type {
        let layout = &self.layout_of(ty).layout.abi();
        debug! {"handling simd with layout {:?}", layout};

        let (element, size) = match layout {
            Vector { element, count } => (element.clone(), count),
            _ => unreachable!(),
        };

        let prim_type = element.primitive();
        let rust_type = self.codegen_prim_typ(prim_type);
        let cbmc_type = self.codegen_ty(rust_type);

        Type::vector(cbmc_type, *size)
    }

    /// the function type of the current instance
    pub fn fn_typ(&mut self) -> Type {
        let sig = self.current_fn().sig();
        let sig =
            self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig.unwrap());
        // we don't call [codegen_function_sig] because we want to get a bit more metainformation.
        let mut params: Vec<Parameter> = sig
            .inputs()
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                if self.ignore_var_ty(*t) {
                    None
                } else {
                    let lc = Local::from_usize(i + 1);
                    let mut ident = self.codegen_var_name(&lc);

                    // `spread_arg` indicates that the last argument is tupled
                    // at the LLVM/codegen level, so we need to declare the indivual
                    // components as parameters with a special naming convention
                    // so that we can "retuple" them in the function prelude.
                    // See: compiler/rustc_codegen_llvm/src/gotoc/mod.rs:codegen_function_prelude
                    if let Some(spread) = self.current_fn().mir().spread_arg {
                        if lc.index() >= spread.index() {
                            let (name, _) = self.codegen_spread_arg_name(&lc);
                            ident = name;
                        }
                    }
                    Some(
                        self.codegen_ty(*t)
                            .as_parameter(Some(ident.clone().into()), Some(ident.into())),
                    )
                }
            })
            .collect();

        // For vtable shims, we need to modify fn(self, ...) to fn(self: *mut Self, ...),
        // since the vtable functions expect a pointer as the first argument. See the comment
        // and similar code in compiler/rustc_mir/src/shim.rs.
        if let ty::InstanceDef::VtableShim(..) = self.current_fn().instance().def {
            if let Some(self_param) = params.first() {
                let ident = self_param.identifier();
                let ty = self_param.typ().clone();
                params[0] = ty.clone().to_pointer().as_parameter(ident, ident);
            }
        }

        debug!(?params, signature=?sig, "function_type");
        if sig.c_variadic {
            Type::variadic_code(params, self.codegen_ty(sig.output()))
        } else {
            Type::code(params, self.codegen_ty(sig.output()))
        }
    }

    /// Whether a variable of type ty should be ignored as a parameter to a function
    pub fn ignore_var_ty(&self, ty: Ty<'tcx>) -> bool {
        match ty.kind() {
            ty::FnDef(_, _) => true,
            // Ignore variables of the generator type until we add support for
            // them:
            // https://github.com/model-checking/kani/issues/416
            ty::Generator(..) => true,
            _ => false,
        }
    }
}

/// Use maps instead of lists to manage mir struct components.
impl<'tcx> GotocCtx<'tcx> {
    /// A mapping from mir field names to mir field types for a mir struct (for a single-variant adt)
    pub fn mir_struct_field_types(
        &self,
        struct_type: Ty<'tcx>,
    ) -> BTreeMap<InternedString, Ty<'tcx>> {
        match struct_type.kind() {
            ty::Adt(adt_def, adt_substs) if adt_def.variants().len() == 1 => {
                let fields = &adt_def.variants().get(VariantIdx::from_u32(0)).unwrap().fields;
                BTreeMap::from_iter(
                    fields.iter().map(|field| {
                        (field.name.to_string().into(), field.ty(self.tcx, adt_substs))
                    }),
                )
            }
            _ => unreachable!("Expected a single-variant ADT. Found {:?}", struct_type),
        }
    }

    /// Extract a trait type from a `Struct<dyn T>`.
    /// Note that `T` must be the last element of the struct.
    /// This also handles nested cases: `Struct<Struct<dyn T>>` returns `dyn T`
    pub fn extract_trait_type(&self, struct_type: Ty<'tcx>) -> Option<Ty<'tcx>> {
        if !self.use_vtable_fat_pointer(struct_type) {
            warn!(got=?struct_type, "Expected trait type or a DST struct with a trait element.");
            return None;
        }

        let mut typ = struct_type;
        while let ty::Adt(adt_def, adt_substs) = typ.kind() {
            assert_eq!(
                adt_def.variants().len(),
                1,
                "Expected a single-variant ADT. Found {:?}",
                typ
            );
            let fields = &adt_def.variants().get(VariantIdx::from_u32(0)).unwrap().fields;
            let last_field = fields.last().expect("Trait should be the last element.");
            typ = last_field.ty(self.tcx, adt_substs);
        }
        if typ.is_trait() { Some(typ) } else { None }
    }
}

/// The mir type is a mir pointer type.
pub fn is_pointer(mir_type: Ty) -> bool {
    return matches!(mir_type.kind(), ty::Ref(..) | ty::RawPtr(..));
}

/// Extract from a mir pointer type the mir type of the value to which the
/// pointer points.
pub fn pointee_type(pointer_type: Ty) -> Option<Ty> {
    match pointer_type.kind() {
        ty::Ref(_, pointee_type, _) => Some(*pointee_type),
        ty::RawPtr(ty::TypeAndMut { ty: pointee_type, .. }) => Some(*pointee_type),
        _ => None,
    }
}

/// Is the MIR type using a C representation (marked with #[repr(C)] at the source level)?
pub fn is_repr_c_adt(mir_type: Ty) -> bool {
    match mir_type.kind() {
        ty::Adt(def, _) => def.repr().c(),
        _ => false,
    }
}

/// This is a place holder function that should normalize the given type.
///
/// TODO: We should normalize the type projection here. For more details, see
/// https://github.com/model-checking/kani/issues/752
fn normalize_type(ty: Ty) -> Ty {
    ty
}

impl<'tcx> GotocCtx<'tcx> {
    /// A pointer to the mir type should be a thin pointer.
    /// Use thin pointer if the type is sized or if the resulting pointer has no metadata.
    pub fn use_thin_pointer(&self, mir_type: Ty<'tcx>) -> bool {
        // ptr_metadata_ty is not defined on all types, the projection of an associated type
        let (metadata, _check_is_sized) = mir_type.ptr_metadata_ty(self.tcx, normalize_type);
        return !self.is_unsized(mir_type) || metadata == self.tcx.types.unit;
    }
    /// A pointer to the mir type should be a slice fat pointer.
    /// We use a slice fat pointer if the metadata is the slice length (type usize).
    pub fn use_slice_fat_pointer(&self, mir_type: Ty<'tcx>) -> bool {
        let (metadata, _check_is_sized) = mir_type.ptr_metadata_ty(self.tcx, normalize_type);
        return metadata == self.tcx.types.usize;
    }
    /// A pointer to the mir type should be a vtable fat pointer.
    /// We use a vtable fat pointer if this is a fat pointer to anything that is not a slice ptr.
    /// I.e.: The metadata is not length (type usize).
    pub fn use_vtable_fat_pointer(&self, mir_type: Ty<'tcx>) -> bool {
        let (metadata, _check_is_sized) = mir_type.ptr_metadata_ty(self.tcx, normalize_type);
        return metadata != self.tcx.types.unit && metadata != self.tcx.types.usize;
    }

    /// Check if the mir type already is a vtable fat pointer.
    pub fn is_vtable_fat_pointer(&self, mir_type: Ty<'tcx>) -> bool {
        self.is_ref_of_unsized(mir_type)
            && self.use_vtable_fat_pointer(pointee_type(mir_type).unwrap())
    }
}
