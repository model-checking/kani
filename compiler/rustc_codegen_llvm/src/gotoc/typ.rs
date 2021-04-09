// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::cbmc::goto_program::{DatatypeComponent, Expr, Symbol, SymbolTable, Type};
use super::cbmc::utils::aggr_name;
use super::metadata::GotocCtx;
use crate::btree_map;
use rustc_ast::ast::Mutability;
use rustc_index::vec::IndexVec;
use rustc_middle::mir::{HasLocalDecls, Local};
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::print::FmtPrinter;
use rustc_middle::ty::subst::{InternalSubsts, SubstsRef};
use rustc_middle::ty::{
    self, AdtDef, Binder, FloatTy, Instance, IntTy, PolyFnSig, TraitRef, Ty, TyS, UintTy,
    VariantDef,
};
use rustc_span;
use rustc_span::def_id::DefId;
use rustc_target::abi::{
    Abi, FieldsShape, Integer, Layout, LayoutOf, Primitive, TagEncoding, VariantIdx, Variants,
};
use std::convert::TryInto;
use std::fmt::Debug;
use tracing::debug;
use ty::layout::HasParamEnv;
/// Map the unit type to an empty struct
///
/// Mapping unit to `void` works for functions with no return type but not for variables with type
/// unit. We treat both uniformly by declaring an empty struct type: `struct Unit {}` and a global
/// variable `struct Unit VoidUnit` returned by all void functions.
const UNIT_TYPE_EMPTY_STRUCT_NAME: &str = "Unit";
pub const FN_RETURN_VOID_VAR_NAME: &str = "VoidUnit";

impl Type {
    pub fn unit() -> Self {
        // We depend on GotocCtx::codegen_ty_unit() to put the type in the symbol table.
        // We don't have access to the symbol table here to do it ourselves.
        Type::struct_tag(UNIT_TYPE_EMPTY_STRUCT_NAME)
    }

    pub fn is_unit(&self) -> bool {
        match self {
            Type::StructTag(name) => *name == aggr_name(UNIT_TYPE_EMPTY_STRUCT_NAME),
            _ => false,
        }
    }

    pub fn is_unit_pointer(&self) -> bool {
        match self {
            Type::Pointer { typ } => typ.is_unit(),
            _ => false,
        }
    }
}

impl Expr {
    pub fn unit(symbol_table: &SymbolTable) -> Self {
        Expr::struct_expr(Type::unit(), btree_map![], symbol_table)
    }

    pub fn is_unit(&self) -> bool {
        self.typ().is_unit()
    }

    pub fn is_unit_pointer(&self) -> bool {
        self.typ().is_unit_pointer()
    }
}

pub fn tuple_fld(n: usize) -> String {
    format!("{}", n)
}

struct StructField<'tcx> {
    idx: u32,
    offset: u64,
    name: String,
    ty: Ty<'tcx>,
}

impl<'tcx> GotocCtx<'tcx> {
    /// Is the MIR type an unsized type (i.e. one represented by a fat pointer?)
    pub fn is_unsized(&self, t: &'tcx TyS<'_>) -> bool {
        !self
            .monomorphize(t)
            .is_sized(self.tcx.at(rustc_span::DUMMY_SP), ty::ParamEnv::reveal_all())
    }

    /// Is the MIR type a ref of an unsized type (i.e. one represented by a fat pointer?)
    pub fn is_ref_of_unsized(&self, t: &'tcx TyS<'_>) -> bool {
        match t.kind() {
            ty::Ref(_, to, _) | ty::RawPtr(ty::TypeAndMut { ty: to, .. }) => self.is_unsized(to),
            _ => false,
        }
    }

    /// Is the MIR type a box of an unsized type (i.e. one represented by a fat pointer?)
    pub fn is_box_of_unsized(&self, t: &'tcx TyS<'_>) -> bool {
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
        def_id: DefId,
        substs: SubstsRef<'tcx>,
    ) -> DatatypeComponent {
        let instance = Instance::resolve(self.tcx, ty::ParamEnv::reveal_all(), def_id, substs)
            .unwrap()
            .unwrap();

        // gives a binder with function signature
        let sig = self.fn_sig_of_instance(instance);

        // gives an Irep Pointer object for the signature
        let fnptr = self.codegen_dynamic_function_sig(sig).to_pointer();

        //DSN For now, use the pretty name not the mangled name.
        let _mangled_fname = self.symbol_name(instance);
        let pretty_fname = self.tcx.item_name(def_id).to_string();

        let ins_ty = instance.ty(self.tcx, ty::ParamEnv::reveal_all());
        let _layout = self.layout_of(ins_ty);

        Type::datatype_component(&pretty_fname, fnptr)
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
    fn codegen_trait_vtable_type(&mut self, t: &'tcx ty::TyS<'tcx>) -> Type {
        self.ensure_struct(&self.vtable_name(t), |ctx, name| ctx.trait_vtable_field_types(t))
    }

    /// a trait dyn Trait is translated to
    /// struct thetrait {
    ///     void* data;
    ///     void* vtable;
    /// }
    fn codegen_trait_fat_ptr_type(&mut self, t: &'tcx ty::TyS<'tcx>) -> Type {
        self.ensure_struct(&self.normalized_trait_name(t), |ctx, name| {
            // At this point in time, the vtable hasn't been codegen yet.
            // However, all we need to know is its name, which we do know.
            // See the comment on codegen_ty_ref.
            let vtable_name = ctx.vtable_name(t);
            vec![
                Type::datatype_component("data", Type::void_pointer()),
                Type::datatype_component("vtable", Type::struct_tag(&vtable_name).to_pointer()),
            ]
        })
    }

    /// Given a trait of type `t`, determine the fields of the struct that will implement it's vtable.
    fn trait_vtable_field_types(&mut self, t: &'tcx ty::TyS<'tcx>) -> Vec<DatatypeComponent> {
        if let ty::Dynamic(binder, _region) = t.kind() {
            // the virtual methods on the trait ref
            let poly = binder.principal().unwrap().with_self_ty(self.tcx, t);
            let mut flds = self
                .tcx
                .vtable_methods(poly)
                .iter()
                .cloned()
                .filter_map(|method| method)
                .map(|(def_id, substs)| self.trait_method_vtable_field_type(def_id, substs))
                .collect();

            let mut vtable_base = vec![
                // TODO: get the correct type for the drop in place. For now, just use void*
                Type::datatype_component("drop", Type::void_pointer()),
                Type::datatype_component("size", Type::size_t()),
                Type::datatype_component("align", Type::size_t()),
            ];
            vtable_base.append(&mut flds);
            vtable_base
        } else {
            unreachable!("Expected to get a dynamic object here");
        }
    }

    /// Given a Binder<TraitRef>, gives back a normalized name for the dynamic type
    /// For example, if we have a `Rectangle` that implements a `Shape`, this will give back
    /// "<Rectangle as Shape>"
    ///
    /// This is used to generate the pretty name of trait methods when building the vtable.
    /// Ideally, we would just use Instance::resolve() to get a defid for a vtable method.
    /// Unfortunately, this doesn't currently work, so instead, we look at the pretty name of the method, and look by that.
    /// As with vtable_name, we have both cases which have "&" and cases which don't.
    /// e.g. "<Rectangle as Shape>" and "<&Rectangle as Shape>".
    /// We solve this by normalizeing and removing the "&">::vol", but the inner type would be <&Rectangle as Vol>
    pub fn normalized_name_of_dynamic_object_type(
        &self,
        trait_ref_t: Binder<TraitRef<'tcx>>,
    ) -> String {
        with_no_trimmed_paths(|| trait_ref_t.skip_binder().to_string().replace("&", ""))
    }

    /// Gives the name for a trait.
    /// In some cases, we have &T, in other cases T, so normalize.
    pub fn normalized_trait_name(&self, t: Ty<'tcx>) -> String {
        assert!(t.is_trait(), "Type {} must be a trait type (a dynamic type)", t);
        self.ty_mangled_name(t).to_string()
    }

    /// Gives the vtable name for a type.
    /// In some cases, we have &T, in other cases T, so normalize.
    pub fn vtable_name(&self, t: Ty<'tcx>) -> String {
        self.normalized_trait_name(t) + "::vtable"
    }

    pub fn ty_mangled_name(&self, t: Ty<'tcx>) -> String {
        use rustc_hir::def::Namespace;
        use rustc_middle::ty::print::Printer;
        let mut name = String::new();
        let printer = FmtPrinter::new(self.tcx, &mut name, Namespace::TypeNS);
        with_no_trimmed_paths(|| printer.print_type(t).unwrap());
        // TODO: The following line is a temporary measure to remove the static lifetime
        // appearing as \'static in mangled type names.  This should be done using regular
        // expressions to handle more or less whitespace around the lifetime, but this
        // requires adding the regex module to the dependencies in Cargo.toml.  This should
        // probably be done modifying the rustc pretty printer, but that is deep in the rustc
        // code.  See the implementation of pretty_print_region on line 1720 in
        // compiler/rustc_middle/src/ty/print/pretty.rs.
        let name = name.replace(" + \'static", "").replace("\'static ", "");
        name
    }

    pub fn enum_union_name(&self, ty: Ty<'tcx>) -> String {
        format!("{}-union", self.ty_mangled_name(ty))
    }

    pub fn enum_case_struct_name(&self, ty: Ty<'tcx>, case: &VariantDef) -> String {
        format!("{}::{}", self.ty_mangled_name(ty), case.ident.name)
    }

    pub fn codegen_ty_raw_array(&mut self, ty: Ty<'tcx>) -> Type {
        match ty.kind() {
            ty::Array(t, c) => {
                let size = self.codegen_const(c, None).int_constant_value().unwrap();
                let elemt = self.codegen_ty(t);
                elemt.array_of(size)
            }
            _ => unreachable!("should only call on array"),
        }
    }

    /// A foreign type is a type that rust does not know the contents of.
    /// We handle this by treating it as an incomplete struct.
    fn codegen_foreign(&mut self, ty: Ty<'tcx>, defid: DefId) -> Type {
        debug!("codegen_foreign {:?} {:?}", ty, defid);
        let name = self.ty_mangled_name(ty);
        self.ensure(&aggr_name(&name), |_ctx, _| Symbol::incomplete_struct(&name));
        Type::struct_tag(&name)
    }

    /// The unit type in Rust is an empty struct in gotoc
    pub fn codegen_ty_unit(&mut self) -> Type {
        self.ensure_struct(UNIT_TYPE_EMPTY_STRUCT_NAME, |_, _| vec![])
    }

    /// codegen for types. it finds a C type which corresponds to a rust type.
    /// that means [ty] has to be monomorphized.
    ///
    /// check [LayoutCx::layout_raw_uncached] for LLVM codegen
    ///
    /// also c.f. https://www.ralfj.de/blog/2020/04/04/layout-debugging.html
    ///      c.f. https://rust-lang.github.io/unsafe-code-guidelines/introduction.html
    pub fn codegen_ty(&mut self, ty: Ty<'tcx>) -> Type {
        if let Some(handler) = self.type_hooks.hook_applies(self.tcx, ty) {
            return handler.handle(self, ty);
        }

        match ty.kind() {
            ty::Int(k) => self.codegen_iint(*k),
            ty::Bool => Type::c_bool(),
            ty::Char => Type::signed_int(32),
            ty::Uint(k) => self.codegen_uint(*k),
            ty::Float(k) => match k {
                FloatTy::F32 => Type::float(),
                FloatTy::F64 => Type::double(),
            },
            ty::Adt(def, _) if def.repr.simd() => self.codegen_vector(ty),
            ty::Adt(def, subst) => {
                debug!("variants are: {:?}", def.variants);
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
                    self.ty_mangled_name(et),
                    len.try_eval_usize(self.tcx, self.param_env()).unwrap()
                );
                // wrap arrays into struct so that one can take advantage of struct copy in C
                //
                // struct [T; n] {
                //   T _0[n];
                // }
                self.ensure_struct(&array_name, |ctx, name| {
                    if et.is_unit() {
                        // we do not generate a struct with an array of units
                        vec![]
                    } else {
                        vec![Type::datatype_component(&0.to_string(), ctx.codegen_ty_raw_array(ty))]
                    }
                })
            }
            //TODO: Ensure that this is correct
            ty::Dynamic(..) => self.codegen_fat_ptr(ty),
            // As per zulip, a raw slice/str is a variable length array
            // https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp
            ty::Slice(e) => self.codegen_ty(e).flexible_array_of(),
            ty::Str => Type::c_char().array_of(0),
            ty::Ref(_, t, _) | ty::RawPtr(ty::TypeAndMut { ty: t, .. }) => self.codegen_ty_ref(t),
            ty::FnDef(_, _) => {
                let sig = self.monomorphize(ty.fn_sig(self.tcx));
                self.codegen_function_sig(sig)
            }
            ty::FnPtr(sig) => self.codegen_function_sig(*sig).to_pointer(),
            ty::Closure(_, subst) => self.codegen_ty_closure(ty, subst),
            ty::Generator(_, _, _) => unimplemented!(),
            ty::Never =>
            // unfortunately, there is no bottom in C. We must pick a type
            {
                Type::empty()
            }
            ty::Tuple(ts) => {
                if ts.is_empty() {
                    self.codegen_ty_unit()
                } else {
                    // we do not have to do two insertions for tuple because it is impossible for
                    // finite tuples to loop.
                    self.ensure_struct(&self.ty_mangled_name(ty), |tcx, name| {
                        tcx.codegen_ty_tuple_fields(name, ty, ts)
                    })
                }
            }
            ty::Projection(_) | ty::Opaque(_, _) => {
                // hidden types that can be revealed by the compiler via normalization
                let normalized = self.tcx.normalize_erasing_regions(ty::ParamEnv::reveal_all(), ty);
                self.codegen_ty(normalized)
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
        name: &str,
        t: Ty<'tcx>,
        substs: ty::subst::SubstsRef<'tcx>,
    ) -> Vec<DatatypeComponent> {
        self.codegen_ty_tuple_like(name, t, substs.iter().map(|g| g.expect_ty()).collect())
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
            Some(Type::datatype_padding(&name, bits))
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
        name: &str,
        flds: Vec<(String, Ty<'tcx>)>,
        layout: &Layout,
        initial_offset: usize,
    ) -> Vec<DatatypeComponent> {
        match &layout.fields {
            FieldsShape::Arbitrary { offsets, memory_index } => {
                let mut fields: Vec<_> = memory_index
                    .iter()
                    .zip(flds)
                    .zip(offsets)
                    .map(|((idx, (n, t)), ofs)| StructField {
                        idx: *idx,
                        offset: ofs.bits(),
                        name: n,
                        ty: t,
                    })
                    .collect();
                // first we determine the order of the fields
                fields.sort_by(|a, b| a.idx.cmp(&b.idx));
                // then we organize all the fields
                let mut final_fields = Vec::with_capacity(fields.len());
                let mut offset: u64 = initial_offset.try_into().unwrap();
                while !fields.is_empty() {
                    let fld = fields.remove(0);
                    // We insert padding, if necessary
                    if let Some(padding) =
                        self.codegen_struct_padding(offset, fld.offset, final_fields.len())
                    {
                        final_fields.push(padding)
                    }
                    // we insert the actual field
                    final_fields.push(Type::datatype_component(&fld.name, self.codegen_ty(fld.ty)));
                    let layout = self.layout_of(fld.ty);
                    // we compute the overall offset of the end of the current struct
                    offset = fld.offset + layout.size.bits();
                }

                // If we don't meet our expected alignment, pad until we do
                let align = layout.align.abi.bits();
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
            _ => unreachable!(),
        }
    }

    fn codegen_ty_tuple_like(
        &mut self,
        name: &str,
        t: Ty<'tcx>,
        tys: Vec<Ty<'tcx>>,
    ) -> Vec<DatatypeComponent> {
        let layout = self.layout_of(t);
        let flds: Vec<_> = tys.iter().enumerate().map(|(i, t)| (tuple_fld(i), *t)).collect();
        // tuple cannot have other initial offset
        self.codegen_struct_fields(name, flds, layout.layout, 0)
    }

    /// a closure is a struct of all its environments
    /// that is, a closure is just a tuple with a unique type identifier, so that Fn related traits
    /// can find its impl.
    fn codegen_ty_closure(&mut self, t: Ty<'tcx>, substs: ty::subst::SubstsRef<'tcx>) -> Type {
        let name = self.ty_mangled_name(t);
        self.ensure_struct(&name, |ctx, name| {
            ctx.codegen_ty_tuple_like(name, t, substs.as_closure().upvar_tys().collect())
        })
    }

    pub fn codegen_fat_ptr(&mut self, t: Ty<'tcx>) -> Type {
        // implement "fat pointers", which are pointers with metadata.
        // to my knowledge, there are three kinds of fat pointers:
        // 1. references to slices
        // 2. references to trait objects.
        // 3. references to structs whose last field is a fat pointer
        match t.kind() {
            ty::Adt(..) if self.is_unsized(t) => {
                // https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp
                let fat_ptr_name = format!("&{}", self.ty_mangled_name(t));
                self.ensure_struct(&fat_ptr_name, |ctx, name| {
                    vec![
                        Type::datatype_component("data", ctx.codegen_ty(t).to_pointer()),
                        Type::datatype_component("len", Type::size_t()),
                    ]
                })
            }
            ty::Slice(e) => {
                // a slice &[t] is translated to
                // struct {
                //   t *data;
                //   usize len;
                // }
                // c.f. core::ptr::FatPtr<T>
                let slice_name = self.ty_mangled_name(t);
                self.ensure_struct(&slice_name, |ctx, _| {
                    vec![
                        Type::datatype_component("data", ctx.codegen_ty(e).to_pointer()),
                        Type::datatype_component("len", Type::size_t()),
                    ]
                })
            }
            ty::Str => self.ensure_struct("str", |_, _| {
                vec![
                    Type::datatype_component("data", Type::c_char().to_pointer()),
                    Type::datatype_component("len", Type::size_t()),
                ]
            }),
            ty::Dynamic(binder, _region) => {
                debug!("type codegen for dynamic with binder {:?} type {:?}", binder, t);
                self.codegen_trait_vtable_type(t);
                self.codegen_trait_fat_ptr_type(t)
            }

            _ => unreachable!(),
        }
    }

    pub fn codegen_ty_ref(&mut self, t: Ty<'tcx>) -> Type {
        // Normalize t to remove projection and opaque types
        let t = self.tcx.normalize_erasing_regions(ty::ParamEnv::reveal_all(), t);

        match t.kind() {
            // These types go to fat pointers
            ty::Dynamic(..) | ty::Slice(_) | ty::Str => self.codegen_fat_ptr(t),
            ty::Adt(..) if self.is_unsized(t) => self.codegen_fat_ptr(t),
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
            | ty::Uint(_) => self.codegen_ty(t).to_pointer(),

            // These types were blocking firecracker. Doing the default thing to unblock.
            // TODO, determine if this is the right course of action
            ty::FnDef(_, _) | ty::Never => self.codegen_ty(t).to_pointer(),

            // These types have no regression tests for them.
            // For soundess, hold off on generating them till we have test-cases.
            ty::Bound(_, _) => todo!("{:?} {:?}", t, t.kind()),
            ty::Error(_) => todo!("{:?} {:?}", t, t.kind()),
            ty::FnPtr(_) => todo!("{:?} {:?}", t, t.kind()),
            ty::Generator(_, _, _) => todo!("{:?} {:?}", t, t.kind()),
            ty::GeneratorWitness(_) => todo!("{:?} {:?}", t, t.kind()),
            ty::Infer(_) => todo!("{:?} {:?}", t, t.kind()),
            ty::Param(_) => todo!("{:?} {:?}", t, t.kind()),
            ty::Placeholder(_) => todo!("{:?} {:?}", t, t.kind()),
        }
    }

    //Dynamic function calls have a first paramater which is the fat-pointer representing a dynamic trait
    //However, the actual call should take a *self. Since we don't know what this is, use `void*` instead.
    pub fn codegen_dynamic_function_sig(&mut self, sig: PolyFnSig<'tcx>) -> Type {
        let sig = self.monomorphize(sig);
        let sig = self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig);
        let mut is_first = true;
        let params = sig
            .inputs()
            .iter()
            .filter_map(|t| {
                if is_first {
                    //TODO assert that this is a dynamic object
                    is_first = false;
                    debug!("The first element in a dynamic function signature had type {:?}", t);
                    Some(Type::void_pointer())
                } else if self.ignore_var_ty(t) {
                    debug!("Ignoring type {:?} in function signature", t);
                    None
                } else {
                    debug!("Using type {:?} in function signature", t);
                    Some(self.codegen_ty(t))
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
                if self.ignore_var_ty(t) {
                    debug!("Ignoring type {:?} in function signature", t);
                    None
                } else {
                    debug!("Using type {:?} in function signature", t);
                    Some(self.codegen_ty(t))
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
        self.ensure_struct(&self.ty_mangled_name(ty), |ctx, name| {
            let variant = &def.variants.raw[0];
            let layout = ctx.layout_of(ty);
            ctx.codegen_variant_struct_fields(name, variant, subst, layout.layout, 0)
        })
    }

    /// generate a struct representing the layout of the variant
    fn codegen_variant_struct_fields(
        &mut self,
        name: &str,
        variant: &VariantDef,
        subst: &'tcx InternalSubsts<'tcx>,
        layout: &Layout,
        initial_offset: usize,
    ) -> Vec<DatatypeComponent> {
        let flds: Vec<_> = variant
            .fields
            .iter()
            .map(|f| (f.ident.name.to_string(), f.ty(self.tcx, subst)))
            .collect();
        self.codegen_struct_fields(name, flds, layout, initial_offset)
    }

    /// codegen unions
    fn codegen_union(
        &mut self,
        ty: Ty<'tcx>,
        def: &'tcx AdtDef,
        subst: &'tcx InternalSubsts<'tcx>,
    ) -> Type {
        self.ensure_union(&self.ty_mangled_name(ty), |ctx, name| {
            def.variants.raw[0]
                .fields
                .iter()
                .map(|f| {
                    Type::datatype_component(
                        &f.ident.name.to_string(),
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
        self.ensure_struct(&self.ty_mangled_name(ty), |ctx, name| {
            // variants appearing in source code (in source code order)
            let source_variants = &adtdef.variants;
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
                            ctx.codegen_variant_struct_fields(name, variant, subst, layout, 0)
                        }
                    }
                }
                Variants::Multiple { tag_encoding, variants, .. } => {
                    match tag_encoding {
                        TagEncoding::Direct => {
                            // direct encoding of tags
                            let discr_t = ctx.codegen_enum_discr_typ(ty);
                            let int = ctx.codegen_ty(discr_t);
                            let discr_offset = ctx.layout_of(discr_t).size.bits_usize();
                            let initial_offset =
                                ctx.variant_min_offset(variants).unwrap_or(discr_offset);
                            let mut fields = vec![Type::datatype_component("case", int)];
                            if let Some(padding) =
                                ctx.codegen_struct_padding(discr_offset, initial_offset, 0)
                            {
                                fields.push(padding);
                            }
                            fields.push(Type::datatype_component(
                                "cases",
                                ctx.codegen_enum_cases_union(
                                    name,
                                    adtdef,
                                    subst,
                                    variants,
                                    initial_offset,
                                ),
                            ));
                            fields
                        }
                        TagEncoding::Niche { dataful_variant, .. } => {
                            // niche encoding is an optimization, which uses invalid values for discriminant
                            // for example, Option<&i32> becomes just a pointer to i32, and pattern
                            // matching becomes checking whether the pointer is null or not. direct
                            // encoding, on the other hand, would have been maintaining a field
                            // storing the discriminant, which is a few bytes larger.
                            //
                            // dataful_variant is pretty much the only variant which contains the valid data
                            let variant = &adtdef.variants[*dataful_variant];
                            ctx.codegen_variant_struct_fields(
                                name,
                                variant,
                                subst,
                                &variants[*dataful_variant],
                                0,
                            )
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
                if lo.fields.count() == 0 { None } else { Some(lo.fields.offset(0).bits_usize()) }
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
            Variants::Multiple { tag, .. } => self.codegen_prim_typ(tag.value),
            _ => unreachable!("only enum has discriminant"),
        }
    }

    fn codegen_enum_cases_union(
        &mut self,
        name: &str,
        def: &'tcx AdtDef,
        subst: &'tcx InternalSubsts<'tcx>,
        layouts: &IndexVec<VariantIdx, Layout>,
        initial_offset: usize,
    ) -> Type {
        self.ensure_union(&format!("{}-union", name), |ctx, name| {
            def.variants
                .iter_enumerated()
                .map(|(i, case)| {
                    Type::datatype_component(
                        &case.ident.name.to_string(),
                        ctx.codegen_enum_case_struct(
                            name,
                            case,
                            subst,
                            &layouts[i],
                            initial_offset,
                        ),
                    )
                })
                .collect()
        })
    }

    fn codegen_enum_case_struct(
        &mut self,
        name: &str,
        case: &VariantDef,
        subst: &'tcx InternalSubsts<'tcx>,
        variant: &Layout,
        initial_offset: usize,
    ) -> Type {
        let case_name = format!("{}::{}", name, case.ident.name);
        debug!("handling variant {}: {:?}", case_name, case);
        self.ensure_struct(&case_name, |tcx, _| {
            tcx.codegen_variant_struct_fields(&case_name, case, subst, variant, initial_offset)
        })
    }

    fn codegen_vector(&mut self, ty: Ty<'tcx>) -> Type {
        let layout = &self.layout_of(ty).layout.abi;
        debug! {"handling simd with layout {:?}", layout};

        let (element, size) = match layout {
            Abi::Vector { element, count } => (element.clone(), *count),
            _ => unreachable!(),
        };

        let rustc_target::abi::Scalar { value: prim_type, .. } = element;
        let rust_type = self.codegen_prim_typ(prim_type);
        let cbmc_type = self.codegen_ty(rust_type);

        Type::vector(cbmc_type, size)
    }

    /// the function type of the current instance
    pub fn fn_typ(&mut self) -> Type {
        let mir = self.mir();
        let sig = self.fn_sig();
        let sig = self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig);
        // we don't call [codegen_function_sig] because we want to get a bit more metainformation.
        let params = sig
            .inputs()
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                if self.ignore_var_ty(t) {
                    None
                } else {
                    let l = Local::from_usize(i + 1);
                    let t = *t;
                    let _ld = &mir.local_decls()[l];
                    let ident = self.codegen_var_name(&l);
                    Some(Type::parameter(Some(ident.to_string()), Some(ident), self.codegen_ty(t)))
                }
            })
            .collect();
        if sig.c_variadic {
            Type::variadic_code(params, self.codegen_ty(sig.output()))
        } else {
            Type::code(params, self.codegen_ty(sig.output()))
        }
    }

    /// whether a variable of type ty should be ignored
    pub fn ignore_var_ty(&self, ty: Ty<'tcx>) -> bool {
        match ty.kind() {
            ty::Tuple(substs) if substs.is_empty() => true,
            ty::FnDef(_, _) => true,
            ty::Dynamic(_, _) => true, //DSN understand why
            _ => false,
        }
    }
}
