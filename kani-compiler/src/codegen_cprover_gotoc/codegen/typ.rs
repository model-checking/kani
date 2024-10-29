// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{DatatypeComponent, Expr, Location, Parameter, Symbol, SymbolTable, Type};
use cbmc::utils::aggr_tag;
use cbmc::{InternString, InternedString};
use rustc_ast::ast::Mutability;
use rustc_index::IndexVec;
use rustc_middle::ty::GenericArgsRef;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::print::FmtPrinter;
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::{
    self, AdtDef, Const, CoroutineArgs, CoroutineArgsExt, FloatTy, Instance, IntTy, PolyFnSig, Ty,
    TyCtxt, TyKind, UintTy, VariantDef, VtblEntry,
};
use rustc_middle::ty::{List, TypeFoldable};
use rustc_smir::rustc_internal;
use rustc_span::def_id::DefId;
use rustc_target::abi::{
    Abi::Vector, FieldIdx, FieldsShape, Float, Integer, LayoutData, Primitive, Size, TagEncoding,
    TyAndLayout, VariantIdx, Variants,
};
use stable_mir::abi::{ArgAbi, FnAbi, PassMode};
use stable_mir::mir::Body;
use stable_mir::mir::mono::Instance as InstanceStable;
use tracing::{debug, trace, warn};

/// Map the unit type to an empty struct
///
/// Mapping unit to `void` works for functions with no return type but not for variables with type
/// unit. We treat both uniformly by declaring an empty struct type: `struct Unit {}` and a global
/// variable `struct Unit VoidUnit` returned by all void functions.
const UNIT_TYPE_EMPTY_STRUCT_NAME: &str = "Unit";
pub const FN_RETURN_VOID_VAR_NAME: &str = "VoidUnit";

/// Name for the common vtable structure.
const COMMON_VTABLE_STRUCT_NAME: &str = "Kani::CommonVTable";

const VTABLE_DROP_FIELD: &str = "drop";
pub const VTABLE_SIZE_FIELD: &str = "size";
pub const VTABLE_ALIGN_FIELD: &str = "align";

/// Map the never i.e. `!` type to an empty struct.
/// The never type can appear as a function argument, e.g. in library/core/src/num/error.rs
const NEVER_TYPE_EMPTY_STRUCT_NAME: &str = "Never";

pub trait TypeExt {
    fn is_rust_fat_ptr(&self, st: &SymbolTable) -> bool;
    fn is_rust_slice_fat_ptr(&self, st: &SymbolTable) -> bool;
    fn is_rust_trait_fat_ptr(&self, st: &SymbolTable) -> bool;
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
                st.lookup(tag.to_string()).unwrap().typ.is_rust_trait_fat_ptr(st)
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
}

/// Function signatures
impl GotocCtx<'_> {
    /// This method prints the details of a GotoC type, for debugging purposes.
    #[allow(unused)]
    pub(crate) fn debug_print_type_recursively(&self, ty: &Type) -> String {
        fn debug_write_type(
            ctx: &GotocCtx,
            ty: &Type,
            out: &mut impl std::fmt::Write,
            indent: usize,
        ) -> Result<(), std::fmt::Error> {
            match ty {
                Type::Array { typ, size } => {
                    write!(out, "[")?;
                    debug_write_type(ctx, typ, out, indent + 2)?;
                    write!(out, "; {size}]")?;
                }
                Type::Bool => write!(out, "bool")?,
                Type::CBitField { typ, width } => {
                    write!(out, "bitfield(")?;
                    debug_write_type(ctx, typ, out, indent + 2)?;
                    write!(out, ", {width}")?;
                }
                Type::CInteger(int_ty) => {
                    let name = match int_ty {
                        cbmc::goto_program::CIntType::Bool => "bool",
                        cbmc::goto_program::CIntType::Char => "char",
                        cbmc::goto_program::CIntType::Int => "int",
                        cbmc::goto_program::CIntType::LongInt => "long int",
                        cbmc::goto_program::CIntType::SizeT => "size_t",
                        cbmc::goto_program::CIntType::SSizeT => "ssize_t",
                    };
                    write!(out, "{name}")?;
                }
                Type::Code { .. } => write!(out, "Code")?,
                Type::Constructor => todo!(),
                Type::Double => write!(out, "f64")?,
                Type::Empty => todo!(),
                Type::FlexibleArray { .. } => todo!(),
                Type::Float => write!(out, "f32")?,
                Type::Float16 => write!(out, "f16")?,
                Type::Float128 => write!(out, "f128")?,
                Type::IncompleteStruct { .. } => todo!(),
                Type::IncompleteUnion { .. } => todo!(),
                Type::InfiniteArray { .. } => todo!(),
                Type::Integer => write!(out, "integer")?,
                Type::Pointer { typ } => {
                    write!(out, "*")?;
                    debug_write_type(ctx, typ, out, indent)?;
                }
                Type::Signedbv { width } => write!(out, "i{width}")?,
                Type::Struct { tag, components } => {
                    let pretty_name = if let Some(symbol) = ctx.symbol_table.lookup(aggr_tag(*tag))
                    {
                        symbol.pretty_name.unwrap()
                    } else {
                        "<no pretty name available>".into()
                    };
                    writeln!(out, "struct {tag} ({pretty_name}) {{")?;
                    for c in components {
                        match c {
                            DatatypeComponent::Field { name, typ } => {
                                write!(out, "{:indent$}{name}: ", "", indent = indent + 2)?;
                                debug_write_type(ctx, typ, out, indent + 2)?;
                                writeln!(out, ",")?;
                            }
                            DatatypeComponent::Padding { bits, .. } => {
                                writeln!(
                                    out,
                                    "{:indent$}/* padding: {bits} bits */",
                                    "",
                                    indent = indent + 2
                                )?;
                            }
                        }
                    }
                    write!(out, "{:indent$}}}", "")?;
                }
                Type::StructTag(tag) => {
                    let ty = &ctx.symbol_table.lookup(*tag).unwrap().typ;
                    debug_write_type(ctx, ty, out, indent)?;
                }
                Type::TypeDef { name, typ } => {
                    write!(out, "typedef {{ {name}: ")?;
                    debug_write_type(ctx, typ, out, indent + 2)?;
                    write!(out, "{:indent$}}}", "")?;
                }
                Type::Union { tag, components } => {
                    let pretty_name = if let Some(symbol) = ctx.symbol_table.lookup(aggr_tag(*tag))
                    {
                        symbol.pretty_name.unwrap()
                    } else {
                        "<no pretty name available>".into()
                    };
                    writeln!(out, "union {tag} ({pretty_name}) {{ ")?;
                    for c in components {
                        match c {
                            DatatypeComponent::Field { name, typ } => {
                                write!(out, "{:indent$}{name}: ", "", indent = indent + 2)?;
                                debug_write_type(ctx, typ, out, indent + 2)?;
                                writeln!(out, ",")?;
                            }
                            DatatypeComponent::Padding { bits, .. } => {
                                writeln!(
                                    out,
                                    "{:indent$}/* padding: {bits} bits */",
                                    "",
                                    indent = indent + 2
                                )?;
                            }
                        }
                    }
                    write!(out, "{:indent$}}}", "")?;
                }
                Type::UnionTag(tag) => {
                    let ty = &ctx.symbol_table.lookup(*tag).unwrap().typ;
                    debug_write_type(ctx, ty, out, indent)?;
                }
                Type::Unsignedbv { width } => write!(out, "u{width}")?,
                Type::VariadicCode { .. } => write!(out, "VariadicCode")?,
                Type::Vector { .. } => todo!(),
            }
            Ok(())
        }
        let mut out = String::new();
        debug_write_type(self, ty, &mut out, 0).unwrap();
        out
    }
}

impl<'tcx> GotocCtx<'tcx> {
    pub fn monomorphize<T>(&self, value: T) -> T
    where
        T: TypeFoldable<TyCtxt<'tcx>>,
    {
        // Instance is Some(..) only when current codegen unit is a function.
        if let Some(current_fn) = &self.current_fn {
            current_fn.instance().instantiate_mir_and_normalize_erasing_regions(
                self.tcx,
                ty::ParamEnv::reveal_all(),
                ty::EarlyBinder::bind(value),
            )
        } else {
            // TODO: confirm with rust team there is no way to monomorphize
            // a global value.
            value
        }
    }

    /// Is the MIR type a zero-sized type.
    pub fn is_zst(&self, t: Ty<'tcx>) -> bool {
        self.layout_of(t).is_zst()
    }

    /// Is the MIR type an unsized type
    /// Unsized types can represent:
    /// 1- Types that rust cannot infer their type such as ForeignItems.
    /// 2- Types that can only be accessed via FatPointer.
    pub fn is_unsized(&self, t: Ty<'tcx>) -> bool {
        !self
            .monomorphize(t)
            .is_sized(*self.tcx.at(rustc_span::DUMMY_SP), ty::ParamEnv::reveal_all())
    }

    /// Generates the type for a single field for a dynamic vtable.
    /// In particular, these fields are function pointers.
    fn trait_method_vtable_field_type(
        &mut self,
        instance: Instance<'tcx>,
        idx: usize,
    ) -> DatatypeComponent {
        // Gives a binder with function signature
        let instance = rustc_internal::stable(instance);

        // Gives an Irep Pointer object for the signature
        let fn_ty = self.codegen_dynamic_function_sig(instance);
        let fn_ptr = fn_ty.to_pointer();

        // vtable field name, i.e., 3_vol (idx_method)
        let vtable_field_name = self.vtable_field_name(idx);

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
        let struct_name = self.vtable_name(t);
        let pretty_name = format!("{}::vtable", self.ty_pretty_name(t));
        self.ensure_struct(&struct_name, pretty_name, |ctx, _| ctx.trait_vtable_field_types(t))
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

    /// Codegen the pointer type for a concrete object that implements the trait object.
    /// I.e.: A trait object is a fat pointer which contains a pointer to a concrete object
    /// and a pointer to its vtable. This method returns a type for the first pointer.
    pub fn codegen_trait_data_pointer(&mut self, typ: ty::Ty<'tcx>) -> Type {
        assert!(self.use_vtable_fat_pointer(typ));
        self.codegen_ty(typ).to_pointer()
    }

    /// A reference to a `Struct<dyn T>` { .., data: T} is translated to
    /// struct RefToTrait {
    ///     `Struct<dyn T>* data`;
    ///     `Metadata<dyn T>* vtable;`
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
        let pretty_name = format!("{}::FatPtr", self.ty_pretty_name(pointee_type));
        let data_type = self.codegen_ty(pointee_type).to_pointer();
        self.ensure_struct(&name, &pretty_name, |ctx, _| {
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
        Type::code_with_unnamed_parameters(vec![self.codegen_ty(t).to_pointer()], Type::unit())
            .to_pointer()
    }

    /// Given a trait of type `t`, determine the fields of the struct that will implement its vtable.
    ///
    /// The order of fields (i.e., the layout of a vtable) is not guaranteed by the compiler.
    /// We follow the order from the `TyCtxt::COMMON_VTABLE_ENTRIES`.
    fn trait_vtable_field_types(&mut self, t: ty::Ty<'tcx>) -> Vec<DatatypeComponent> {
        let mut vtable_base = common_vtable_fields(self.trait_vtable_drop_type(t));
        if let ty::Dynamic(binder, _, _) = t.kind() {
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
            debug!(ty=?t, ?vtable_base, "trait_vtable_field_types");
            vtable_base
        } else {
            unreachable!("Expected to get a dynamic object here");
        }
    }

    /// Gives the name for a trait, i.e., `dyn T`. This does not work for `&dyn T`.
    pub fn normalized_trait_name(&self, t: Ty<'tcx>) -> String {
        assert!(t.is_trait(), "Type {t} must be a trait type (a dynamic type)");
        self.ty_mangled_name(t).to_string()
    }

    /// Gives the vtable name for a type.
    /// In some cases, we have &T, in other cases T, so normalize.
    ///
    /// TODO: to handle trait upcasting, this will need to use a
    /// poly existential trait type as a part of the key as well.
    /// See compiler/rustc_middle/src/ty/vtable.rs
    /// <https://github.com/model-checking/kani/issues/358>
    pub fn vtable_name(&self, t: Ty<'tcx>) -> String {
        format!("{}::vtable", self.normalized_trait_name(t))
    }

    pub fn ty_pretty_name(&self, t: Ty<'tcx>) -> InternedString {
        use crate::rustc_middle::ty::print::Print;
        use rustc_hir::def::Namespace;
        let mut printer = FmtPrinter::new(self.tcx, Namespace::TypeNS);

        // Monomorphizing the type ensures we get a cannonical form for dynamic trait
        // objects with auto traits, such as:
        //   StructTag("tag-std::boxed::Box<(dyn std::error::Error + std::marker::Send + std::marker::Sync)>") }
        //   StructTag("tag-std::boxed::Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>") }
        let t = self.monomorphize(t);
        t.print(&mut printer).unwrap();
        with_no_trimmed_paths!(printer.into_buffer()).intern()
    }

    pub fn ty_mangled_name(&self, t: Ty<'tcx>) -> InternedString {
        // Crate resolution: mangled names need to be distinct across different versions
        // of the same crate that could be pulled in by dependencies. However, Kani's
        // treatment of FFI C calls assumes that we generate the same name for #[repr(C)] types
        // as the C name, so don't mangle in that case.
        // However, there was an issue with different type instantiations being given the same mangled name.
        // https://github.com/model-checking/kani/issues/1438.
        // Hence we DO mangle the type name if the type has generic type arguments, even if it's #[repr(C)].
        // This is not a restriction because C can only access non-generic types anyway.
        // TODO: Skipping name mangling is likely insufficient if a dependent crate has two versions of
        // linked C libraries
        // https://github.com/model-checking/kani/issues/450
        match t.kind() {
            TyKind::Adt(def, args) if args.is_empty() && def.repr().c() => {
                // For non-generic #[repr(C)] types, use the literal path instead of mangling it.
                self.tcx.def_path_str(def.did()).intern()
            }
            _ => {
                // This hash is documented to be the same no matter the crate context
                let id = self.tcx.type_id_hash(t).as_u128();
                format!("_{id}").intern()
            }
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

    fn codegen_ty_raw_array(&mut self, elem_ty: Ty<'tcx>, len: Const<'tcx>) -> Type {
        let size = self.codegen_const_internal(len, None).int_constant_value().unwrap();
        let elemt = self.codegen_ty(elem_ty);
        elemt.array_of(size)
    }

    /// A foreign type is a type that rust does not know the contents of.
    /// We handle this by treating it as an incomplete struct.
    fn codegen_foreign(&mut self, ty: Ty<'tcx>, defid: DefId) -> Type {
        debug!("codegen_foreign {:?} {:?}", ty, defid);
        let name = self.ty_mangled_name(ty).intern();
        self.ensure(aggr_tag(name), |ctx, _| {
            Symbol::incomplete_struct(name, ctx.ty_pretty_name(ty))
        });
        Type::struct_tag(name)
    }

    /// Codegens the an initalizer for variables without one.
    /// By default, returns `None` which leaves the variable uninitilized.
    /// In CBMC, this translates to a NONDET value.
    /// In the future, we might want to replace this with `Poison`.
    pub fn codegen_default_initializer(&self, _e: &Expr) -> Option<Expr> {
        None
    }

    /// The unit type in Rust is an empty struct in gotoc
    pub fn codegen_ty_unit(&mut self) -> Type {
        self.ensure_struct(UNIT_TYPE_EMPTY_STRUCT_NAME, "()", |_, _| vec![])
    }

    /// The common VTable entries.
    /// A VTable is an opaque type to the compiler, but they all follow the same structure.
    /// The first three entries are always the following:
    /// 1- Function pointer to drop in place.
    /// 2- The size of the object.
    /// 3- The alignment of the object.
    /// We use this common structure to extract information out of a vtable. Since we don't have
    /// any information about the original type, we use `void*` to encode the drop fn argument type.
    pub fn codegen_ty_common_vtable(&mut self) -> Type {
        self.ensure_struct(COMMON_VTABLE_STRUCT_NAME, COMMON_VTABLE_STRUCT_NAME, |_, _| {
            let drop_type =
                Type::code_with_unnamed_parameters(vec![Type::void_pointer()], Type::unit())
                    .to_pointer();
            common_vtable_fields(drop_type)
        })
    }

    /// codegen for types. it finds a C type which corresponds to a rust type.
    /// that means [ty] has to be monomorphized before calling this function.
    ///
    /// check `rustc_ty_utils::layout::layout_of_uncached` for LLVM codegen
    ///
    /// also c.f. <https://www.ralfj.de/blog/2020/04/04/layout-debugging.html>
    ///      c.f. <https://rust-lang.github.io/unsafe-code-guidelines/introduction.html>
    pub fn codegen_ty(&mut self, ty: Ty<'tcx>) -> Type {
        // TODO: Remove all monomorphize calls
        let normalized = self.tcx.normalize_erasing_regions(ty::ParamEnv::reveal_all(), ty);
        let goto_typ = self.codegen_ty_inner(normalized);
        if let Some(tag) = goto_typ.tag() {
            self.type_map.entry(tag).or_insert_with(|| {
                debug!(mir_type=?normalized, gotoc_name=?tag, ?goto_typ,  "codegen_ty: new type");
                normalized
            });
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
                FloatTy::F16 => Type::float16(),
                FloatTy::F128 => Type::float128(),
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
            ty::Array(et, len) => self.codegen_ty_raw_array(*et, *len),
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
            ty::Str => Type::unsigned_int(8).flexible_array_of(),
            ty::Ref(_, t, _) | ty::RawPtr(t, _) => self.codegen_ty_ref(*t),
            ty::FnDef(def_id, args) => {
                let instance =
                    Instance::try_resolve(self.tcx, ty::ParamEnv::reveal_all(), *def_id, args)
                        .unwrap()
                        .unwrap();
                self.codegen_fndef_type(instance)
            }
            ty::FnPtr(sig_tys, hdr) => {
                let sig = sig_tys.with(*hdr);
                self.codegen_function_sig(sig).to_pointer()
            }
            ty::Closure(_, subst) => self.codegen_ty_closure(ty, subst),
            ty::Coroutine(..) => self.codegen_ty_coroutine(ty),
            ty::Never => self.ensure_struct(NEVER_TYPE_EMPTY_STRUCT_NAME, "!", |_, _| vec![]),
            ty::Tuple(ts) => {
                if ts.is_empty() {
                    self.codegen_ty_unit()
                } else {
                    // we do not have to do two insertions for tuple because it is impossible for
                    // finite tuples to loop.
                    self.ensure_struct(
                        self.ty_mangled_name(ty),
                        self.ty_pretty_name(ty),
                        |tcx, _| tcx.codegen_ty_tuple_fields(ty, ts),
                    )
                }
            }
            // This object has the same layout as base. For now, translate this into `(base)`.
            // The only difference is the niche.
            ty::Pat(base_ty, ..) => {
                self.ensure_struct(self.ty_mangled_name(ty), self.ty_pretty_name(ty), |tcx, _| {
                    tcx.codegen_ty_tuple_like(ty, vec![*base_ty])
                })
            }
            ty::Alias(..) => {
                unreachable!("Type should've been normalized already")
            }

            // shouldn't come to here after mormomorphization
            ty::Bound(_, _) | ty::Param(_) => unreachable!("monomorphization bug"),

            // type checking remnants which shouldn't be reachable
            ty::CoroutineWitness(_, _)
            | ty::CoroutineClosure(_, _)
            | ty::Infer(_)
            | ty::Placeholder(_)
            | ty::Error(_) => {
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

    fn codegen_struct_padding(
        &self,
        current_offset: Size,
        next_offset: Size,
        idx: usize,
    ) -> Option<DatatypeComponent> {
        assert!(current_offset <= next_offset);
        if current_offset < next_offset {
            // We need to pad to the next offset
            let padding_size = next_offset - current_offset;
            let name = format!("$pad{idx}");
            Some(DatatypeComponent::padding(name, padding_size.bits()))
        } else {
            None
        }
    }

    /// Adds padding to ensure that the size of the struct is a multiple of the alignment
    fn codegen_alignment_padding(
        &self,
        size: Size,
        layout: &LayoutData<FieldIdx, VariantIdx>,
        idx: usize,
    ) -> Option<DatatypeComponent> {
        let align = Size::from_bits(layout.align.abi.bits());
        let overhang = Size::from_bits(size.bits() % align.bits());
        if overhang != Size::ZERO {
            self.codegen_struct_padding(size, size + align - overhang, idx)
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
        layout: &LayoutData<FieldIdx, VariantIdx>,
        initial_offset: Size,
    ) -> Vec<DatatypeComponent> {
        match &layout.fields {
            FieldsShape::Arbitrary { offsets, memory_index } => {
                assert_eq!(flds.len(), offsets.len());
                assert_eq!(offsets.len(), memory_index.len());
                let mut final_fields = Vec::with_capacity(flds.len());
                let mut offset = initial_offset;
                for idx in layout.fields.index_by_increasing_offset() {
                    let fld_offset = offsets[idx.into()];
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
                    offset = fld_offset + layout.size;
                }
                final_fields.extend(self.codegen_alignment_padding(
                    offset,
                    layout,
                    final_fields.len(),
                ));
                final_fields
            }
            // Primitives, such as NEVER, have no fields
            FieldsShape::Primitive => vec![],
            _ => unreachable!("{}\n{:?}", self.current_fn().readable_name(), layout.fields),
        }
    }

    fn codegen_ty_tuple_like(&mut self, t: Ty<'tcx>, tys: Vec<Ty<'tcx>>) -> Vec<DatatypeComponent> {
        let layout = self.layout_of(t);
        let flds: Vec<_> =
            tys.iter().enumerate().map(|(i, t)| (GotocCtx::tuple_fld_name(i), *t)).collect();
        // tuple cannot have other initial offset
        self.codegen_struct_fields(flds, &layout.layout.0, Size::ZERO)
    }

    /// A closure is a struct of all its environments. That is, a closure is
    /// just a tuple with a unique type identifier, so that Fn related traits
    /// can find its impl.
    fn codegen_ty_closure(&mut self, t: Ty<'tcx>, args: ty::GenericArgsRef<'tcx>) -> Type {
        self.ensure_struct(self.ty_mangled_name(t), self.ty_pretty_name(t), |ctx, _| {
            ctx.codegen_ty_tuple_like(t, args.as_closure().upvar_tys().to_vec())
        })
    }

    /// Translate a coroutine type similarly to an enum with a variant for each suspend point.
    ///
    /// Consider the following coroutine:
    /// ```
    /// || {
    ///     let a = true;
    ///     let b = &a;
    ///     yield;
    ///     assert_eq!(b as *const _, &a as *const _);
    ///     yield;
    /// };
    /// ```
    ///
    /// Rustc compiles this to something similar to the following enum (but there are differences, see below!),
    /// as described at the top of <https://github.com/rust-lang/rust/blob/master/compiler/rustc_mir_transform/src/coroutine.rs>:
    ///
    /// ```ignore
    /// enum CoroutineEnum {
    ///     Unresumed,                        // initial state of the coroutine
    ///     Returned,                         // coroutine has returned
    ///     Panicked,                         // coroutine has panicked
    ///     Suspend0 { b: &bool, a: bool },   // state after suspending (`yield`ing) for the first time
    ///     Suspend1,                         // state after suspending (`yield`ing) for the second time
    /// }
    /// ```
    ///
    /// However, its layout may differ from normal Rust enums in the following ways:
    /// * Contrary to enums, the discriminant may not be at offset 0.
    /// * Contrary to enums, there may be other fields than the discriminant "at the top level" (outside the variants).
    ///
    /// This means that we CANNOT use the enum translation, which would be roughly as follows:
    ///
    /// ```ignore
    /// struct CoroutineEnum {
    ///     int case; // discriminant
    ///     union CoroutineEnum-union cases; // variant
    /// }
    ///
    /// union CoroutineEnum-union {
    ///     struct Unresumed variant0;
    ///     struct Returned variant1;
    ///     // ...
    /// }
    /// ```
    ///
    /// Instead, we use the following translation:
    ///
    /// ```ignore
    /// union CoroutineEnum {
    ///     struct DirectFields direct_fields;
    ///     struct Unresumed coroutine_variant_Unresumed;
    ///     struct Returned coroutine_variant_Returned;
    ///     // ...
    /// }
    ///
    /// struct DirectFields {
    ///     // padding (for bool *b in Suspend0 below)
    ///     char case;
    ///     // padding (for bool a in Suspend0 below)
    /// }
    ///
    /// struct Unresumed {
    ///     // padding (this variant has no fields)
    /// }
    ///
    /// // ...
    ///
    /// struct Suspend0 {
    ///     bool *coroutine_field_0; // variable b in the coroutine code above
    ///     // padding (for char case in DirectFields)
    ///     bool coroutine_field_1; // variable a in the coroutine code above
    /// }
    /// ```
    ///
    /// Of course, if the coroutine has any other top-level/direct fields, they'd be included in the `DirectFields` struct as well.
    fn codegen_ty_coroutine(&mut self, ty: Ty<'tcx>) -> Type {
        let coroutine_name = self.ty_mangled_name(ty);
        let pretty_name = self.ty_pretty_name(ty);
        debug!(?pretty_name, "codeged_ty_coroutine");
        self.ensure_union(self.ty_mangled_name(ty), pretty_name, |ctx, _| {
            let type_and_layout = ctx.layout_of(ty);
            let (discriminant_field, variants) = match &type_and_layout.variants {
                Variants::Multiple {
                    tag_encoding: TagEncoding::Direct,
                    tag_field,
                    variants,
                    ..
                } => (tag_field, variants),
                _ => unreachable!("Coroutines have more than one variant and use direct encoding"),
            };
            // generate a struct for the direct fields of the layout (fields that don't occur in the variants)
            let direct_fields = DatatypeComponent::Field {
                name: "direct_fields".into(),
                typ: ctx.codegen_coroutine_variant_struct(
                    coroutine_name,
                    pretty_name,
                    type_and_layout,
                    "DirectFields".into(),
                    Some(*discriminant_field),
                ),
            };
            let mut fields = vec![direct_fields];
            for var_idx in variants.indices() {
                let variant_name = CoroutineArgs::variant_name(var_idx).into();
                fields.push(DatatypeComponent::Field {
                    name: ctx.coroutine_variant_name(var_idx),
                    typ: ctx.codegen_coroutine_variant_struct(
                        coroutine_name,
                        pretty_name,
                        type_and_layout.for_variant(ctx, var_idx),
                        variant_name,
                        None,
                    ),
                });
            }
            fields
        })
    }

    /// Generates a struct for a variant of the coroutine.
    ///
    /// The field `discriminant_field` should be `Some(idx)` when generating the variant for the direct (top-[evel) fields of the coroutine.
    /// Then the field with the index `idx` will be treated as the discriminant and will be given a special name to work with the rest of the code.
    /// The field `discriminant_field` should be `None` when generating an actual variant of the coroutine because those don't contain the discriminant as a field.
    fn codegen_coroutine_variant_struct(
        &mut self,
        coroutine_name: InternedString,
        pretty_coroutine_name: InternedString,
        type_and_layout: TyAndLayout<'tcx, Ty<'tcx>>,
        variant_name: InternedString,
        discriminant_field: Option<usize>,
    ) -> Type {
        let struct_name = format!("{coroutine_name}::{variant_name}");
        let pretty_struct_name = format!("{pretty_coroutine_name}::{variant_name}");
        debug!(?pretty_struct_name, "codeged_coroutine_variant_struct");
        self.ensure_struct(struct_name, pretty_struct_name, |ctx, _| {
            let mut offset = Size::ZERO;
            let mut fields = vec![];
            for idx in type_and_layout.fields.index_by_increasing_offset() {
                // The discriminant field needs to have a special name to work with the rest of the code.
                // If discriminant_field is None, this variant does not have the discriminant as a field.
                let field_name = if Some(idx) == discriminant_field {
                    "case".into()
                } else {
                    ctx.coroutine_field_name(idx)
                };
                let field_ty = type_and_layout.field(ctx, idx).ty;
                let field_offset = type_and_layout.fields.offset(idx);
                let field_size = type_and_layout.field(ctx, idx).size;
                if let Some(padding) = ctx.codegen_struct_padding(offset, field_offset, idx) {
                    fields.push(padding);
                }
                fields.push(DatatypeComponent::Field {
                    name: field_name,
                    typ: ctx.codegen_ty(field_ty),
                });
                offset = field_offset + field_size;
            }
            fields.extend(ctx.codegen_alignment_padding(
                offset,
                &type_and_layout.layout.0,
                fields.len(),
            ));
            fields
        })
    }

    pub fn coroutine_variant_name(&self, var_idx: VariantIdx) -> InternedString {
        format!("coroutine_variant_{}", CoroutineArgs::variant_name(var_idx)).into()
    }

    pub fn coroutine_field_name(&self, field_idx: usize) -> InternedString {
        format!("coroutine_field_{field_idx}").into()
    }

    /// Codegen "fat pointers" to the given `pointee_type`. These are pointers with metadata.
    ///
    /// There are three kinds of fat pointers:
    /// 1. references to slices (`matches!(pointee_type.kind(), ty::Slice(..) | ty::Str)`).
    /// 2. references to trait objects (`matches!(pointee_type.kind(), ty::Dynamic)`).
    /// 3. references to structs whose last field is a unsized object (slice / trait)
    ///    - `matches!(pointee_type.kind(), ty::Adt(..) if self.is_unsized(t))
    ///
    fn codegen_fat_ptr(&mut self, pointee_type: Ty<'tcx>) -> Type {
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
            let pretty_name = format!("&{}", self.ty_pretty_name(pointee_type));
            let element_type = match pointee_type.kind() {
                ty::Slice(elt_type) => self.codegen_ty(*elt_type),
                ty::Str => Type::unsigned_int(8),
                // For adt, see https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp
                ty::Adt(..) => self.codegen_ty(pointee_type),
                kind => unreachable!("Generating a slice fat pointer to {:?}", kind),
            };
            self.ensure_struct(pointer_name, pretty_name, |_, _| {
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
            ty::Alias(..) => {
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
            | ty::Coroutine(..)
            | ty::Int(_)
            | ty::RawPtr(_, _)
            | ty::Ref(..)
            | ty::Pat(..)
            | ty::Tuple(_)
            | ty::Uint(_) => self.codegen_ty(pointee_type).to_pointer(),

            // These types were blocking firecracker. Doing the default thing to unblock.
            // https://github.com/model-checking/kani/issues/215
            // https://github.com/model-checking/kani/issues/216
            ty::FnDef(_, _) | ty::Never => self.codegen_ty(pointee_type).to_pointer(),

            // These types were blocking stdlib. Doing the default thing to unblock.
            // https://github.com/model-checking/kani/issues/214
            ty::FnPtr(_, _) => self.codegen_ty(pointee_type).to_pointer(),

            // These types have no regression tests for them.
            // For soundness, hold off on generating them till we have test-cases.
            ty::Bound(_, _) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::Error(_) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::CoroutineWitness(_, _) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::CoroutineClosure(_, _) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::Infer(_) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::Param(_) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
            ty::Placeholder(_) => todo!("{:?} {:?}", pointee_type, pointee_type.kind()),
        }
    }

    /// Generate code for a trait function declaration.
    ///
    /// Dynamic function calls first parameter is self which must be one of the following:
    ///
    /// As of Jul 2022:
    /// `P = &Self | &mut Self | Box<Self> | Rc<Self> | Arc<Self>`
    /// `S = P | Pin<P>`
    ///
    /// See <https://doc.rust-lang.org/reference/items/traits.html#object-safety> for more details.
    fn codegen_dynamic_function_sig(&mut self, instance: InstanceStable) -> Type {
        let mut is_first = true;
        let fn_abi = instance.fn_abi().unwrap();
        let args = self.codegen_args(instance, &fn_abi);
        let params = args
            .map(|(_, arg_abi)| {
                let arg_ty_stable = arg_abi.ty;
                let kind = arg_ty_stable.kind();
                let arg_ty = rustc_internal::internal(self.tcx, arg_ty_stable);
                if is_first {
                    is_first = false;
                    debug!(self_type=?arg_ty, ?fn_abi, "codegen_dynamic_function_sig");
                    if kind.is_ref() {
                        // Convert fat pointer to thin pointer to data portion.
                        let first_ty = pointee_type(arg_ty).unwrap();
                        self.codegen_trait_data_pointer(first_ty)
                    } else if kind.is_trait() {
                        // Convert dyn T to thin pointer.
                        self.codegen_trait_data_pointer(arg_ty)
                    } else {
                        // Codegen type with thin pointer (E.g.: Box<dyn T> -> Box<data_ptr>).
                        self.codegen_trait_receiver(arg_ty)
                    }
                } else {
                    debug!("Using type {:?} in function signature", arg_ty);
                    self.codegen_ty(arg_ty)
                }
            })
            .collect();

        Type::code_with_unnamed_parameters(params, self.codegen_ty_stable(fn_abi.ret.ty))
    }

    /// one can only apply this function to a monomorphized signature
    pub fn codegen_function_sig(&mut self, sig: PolyFnSig<'tcx>) -> Type {
        let sig = self.monomorphize(sig);
        let sig = self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig);
        self.codegen_function_sig_stable(rustc_internal::stable(sig))
    }

    /// Creates a zero-sized struct for a FnDef.
    ///
    /// A FnDef instance in Rust is a zero-sized type, which can be passed around directly, without creating a pointer.
    /// (Rust docs: <https://doc.rust-lang.org/reference/types/function-item.html>)
    /// To mirror this in GotoC, we create a dummy struct for the function, similarly to what we do for closures.
    ///
    /// For details, see <https://github.com/model-checking/kani/pull/1338>
    pub fn codegen_fndef_type(&mut self, instance: Instance<'tcx>) -> Type {
        self.codegen_fndef_type_stable(rustc_internal::stable(instance))
    }

    /// codegen for struct
    ///
    /// they are literally codegen'ed in the corresponding way (except the order of fields might not be preserved)
    fn codegen_struct(
        &mut self,
        ty: Ty<'tcx>,
        def: &'tcx AdtDef,
        subst: &'tcx GenericArgsRef<'tcx>,
    ) -> Type {
        self.ensure_struct(self.ty_mangled_name(ty), self.ty_pretty_name(ty), |ctx, _| {
            let variant = &def.variants().raw[0];
            let layout = ctx.layout_of(ty);
            ctx.codegen_variant_struct_fields(variant, subst, &layout.layout.0, Size::ZERO)
        })
    }

    /// generate a struct representing the layout of the variant
    fn codegen_variant_struct_fields(
        &mut self,
        variant: &VariantDef,
        subst: &'tcx GenericArgsRef<'tcx>,
        layout: &LayoutData<FieldIdx, VariantIdx>,
        initial_offset: Size,
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
        subst: &'tcx GenericArgsRef<'tcx>,
    ) -> Type {
        self.ensure_union(self.ty_mangled_name(ty), self.ty_pretty_name(ty), |ctx, _| {
            def.variants().raw[0]
                .fields
                .iter()
                .map(|f| {
                    DatatypeComponent::field(
                        f.name.to_string(),
                        ctx.codegen_ty(f.ty(ctx.tcx, subst)),
                    )
                })
                .collect()
        })
    }

    /// Mapping enums to CBMC types is rather complicated. There are a few cases to consider:
    /// 1. When there is only 0 or 1 variant, this is straightforward as the code shows
    /// 2. When there are more variants, rust might decide to apply the typical encoding which
    ///    regard enums as tagged union, or an optimized form, called niche encoding.
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
    /// c.f. <https://rust-lang.github.io/unsafe-code-guidelines/layout/enums.html#layout-of-a-data-carrying-enums-without-a-repr-annotation>
    fn codegen_enum(
        &mut self,
        ty: Ty<'tcx>,
        adtdef: &'tcx AdtDef,
        subst: &'tcx GenericArgsRef<'tcx>,
    ) -> Type {
        let pretty_name = self.ty_pretty_name(ty);
        // variants appearing in source code (in source code order)
        let source_variants = &adtdef.variants();
        let layout = self.layout_of(ty);
        // variants appearing in mir code
        match &layout.variants {
            Variants::Single { index } => {
                self.ensure_struct(self.ty_mangled_name(ty), pretty_name, |gcx, _| {
                    match source_variants.get(*index) {
                        None => {
                            // an empty enum with no variants (its value cannot be instantiated)
                            vec![]
                        }
                        Some(variant) => {
                            // a single enum is pretty much like a struct
                            let layout = gcx.layout_of(ty).layout;
                            gcx.codegen_variant_struct_fields(variant, subst, &layout.0, Size::ZERO)
                        }
                    }
                })
            }
            Variants::Multiple { tag_encoding, variants, tag_field, .. } => {
                // Contrary to coroutines, currently enums have only one field (the discriminant), the rest are in the variants:
                assert!(layout.fields.count() <= 1);
                // Contrary to coroutines, the discriminant is the first (and only) field for enums:
                assert_eq!(*tag_field, 0);
                match tag_encoding {
                    TagEncoding::Direct => {
                        self.ensure_struct(self.ty_mangled_name(ty), pretty_name, |gcx, name| {
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
                            let discr_t = gcx.codegen_enum_discr_typ(ty);
                            let int = gcx.codegen_ty(discr_t);
                            let discr_offset = gcx.layout_of(discr_t).size;
                            let initial_offset =
                                gcx.variant_min_offset(variants).unwrap_or(discr_offset);
                            let mut fields = vec![DatatypeComponent::field("case", int)];
                            if let Some(padding) =
                                gcx.codegen_struct_padding(discr_offset, initial_offset, 0)
                            {
                                fields.push(padding);
                            }
                            let union_name = format!("{name}-union");
                            let union_pretty_name = format!("{pretty_name}-union");
                            fields.push(DatatypeComponent::field(
                                "cases",
                                gcx.ensure_union(&union_name, &union_pretty_name, |ctx, name| {
                                    ctx.codegen_enum_cases(
                                        name,
                                        pretty_name,
                                        adtdef,
                                        subst,
                                        variants,
                                        initial_offset,
                                    )
                                }),
                            ));
                            // Check if any padding is needed for alignment. This is needed for
                            // https://github.com/model-checking/kani/issues/2857 for example.
                            // The logic for determining the maximum variant size is taken from:
                            // https://github.com/rust-lang/rust/blob/e60ebb2f2c1facba87e7971798f3cbdfd309cd23/compiler/rustc_session/src/code_stats.rs#L166
                            let max_variant_size = variants
                                .iter()
                                .map(|l: &LayoutData<FieldIdx, VariantIdx>| l.size)
                                .max()
                                .unwrap();
                            let max_variant_size = std::cmp::max(max_variant_size, discr_offset);
                            if let Some(padding) = gcx.codegen_alignment_padding(
                                max_variant_size,
                                &layout,
                                fields.len(),
                            ) {
                                fields.push(padding);
                            }
                            fields
                        })
                    }
                    TagEncoding::Niche { .. } => {
                        self.codegen_enum_niche(ty, adtdef, subst, variants)
                    }
                }
            }
        }
    }

    /// Codegen an enumeration that is encoded using niche optimization.
    ///
    /// Enumerations with multiple variants and niche encoding have a
    /// specific format that can be used to optimize its layout and reduce
    /// memory consumption.
    ///
    /// The niche is a location in the entire type where some bit pattern
    /// isn't valid. The compiler uses the `untagged_variant` index to
    /// access this field.
    /// The final size and alignment is also equal to the one from the
    /// `untagged_variant`. All other variants either don't have any field,
    /// or their size is smaller than the `untagged_variant`.
    /// See <https://github.com/rust-lang/rust/issues/46213> for more details.
    ///
    /// Because of that, we usually represent these enums as simple unions
    /// where each field represent one variant. This allows them to be
    /// referred to correctly.
    ///
    /// The one exception is the case where only one variant has data.
    /// We use a struct instead because it is more performant.
    fn codegen_enum_niche(
        &mut self,
        ty: Ty<'tcx>,
        adtdef: &'tcx AdtDef,
        subst: &'tcx GenericArgsRef<'tcx>,
        variants: &IndexVec<VariantIdx, LayoutData<FieldIdx, VariantIdx>>,
    ) -> Type {
        let non_zst_count = variants.iter().filter(|layout| layout.size.bytes() > 0).count();
        let mangled_name = self.ty_mangled_name(ty);
        let pretty_name = self.ty_pretty_name(ty);
        tracing::trace!(?pretty_name, ?variants, ?subst, ?non_zst_count, "codegen_enum: Niche");
        if non_zst_count > 1 {
            self.ensure_union(mangled_name, pretty_name, |gcx, name| {
                gcx.codegen_enum_cases(name, pretty_name, adtdef, subst, variants, Size::ZERO)
            })
        } else {
            self.ensure_struct(mangled_name, pretty_name, |gcx, name| {
                gcx.codegen_enum_cases(name, pretty_name, adtdef, subst, variants, Size::ZERO)
            })
        }
    }

    pub(crate) fn variant_min_offset(
        &self,
        variants: &IndexVec<VariantIdx, LayoutData<FieldIdx, VariantIdx>>,
    ) -> Option<Size> {
        variants
            .iter()
            .filter_map(|lo| {
                if lo.fields.count() == 0 {
                    None
                } else {
                    // get the offset of the leftmost field, which is the one
                    // with the least offset since we codegen fields in a struct
                    // in the order of increasing offsets. Note that this is not
                    // necessarily the 0th field since the compiler may reorder
                    // fields.
                    Some(lo.fields.offset(lo.fields.index_by_increasing_offset().next().unwrap()))
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
            Primitive::Float(f) => self.codegen_float_type(f),
            Primitive::Pointer(_) => Ty::new_ptr(self.tcx, self.tcx.types.u8, Mutability::Not),
        }
    }

    pub fn codegen_float_type(&self, f: Float) -> Ty<'tcx> {
        match f {
            Float::F32 => self.tcx.types.f32,
            Float::F64 => self.tcx.types.f64,
            // `F16` and `F128` are not yet handled.
            // Tracked here: <https://github.com/model-checking/kani/issues/3069>
            Float::F16 | Float::F128 => unimplemented!(),
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
        pretty_name: InternedString,
        def: &'tcx AdtDef,
        subst: &'tcx GenericArgsRef<'tcx>,
        layouts: &IndexVec<VariantIdx, LayoutData<FieldIdx, VariantIdx>>,
        initial_offset: Size,
    ) -> Vec<DatatypeComponent> {
        def.variants()
            .iter_enumerated()
            .filter_map(|(i, case)| {
                if case.fields.is_empty() {
                    // Skip variant types that cannot be referenced.
                    None
                } else {
                    Some(DatatypeComponent::field(
                        case.name.to_string(),
                        self.codegen_enum_case_struct(
                            name,
                            pretty_name,
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
        pretty_name: InternedString,
        case: &VariantDef,
        subst: &'tcx GenericArgsRef<'tcx>,
        variant: &LayoutData<FieldIdx, VariantIdx>,
        initial_offset: Size,
    ) -> Type {
        let case_name = format!("{name}::{}", case.name);
        let pretty_name = format!("{pretty_name}::{}", case.name);
        debug!("handling variant {}: {:?}", case_name, case);
        self.ensure_struct(&case_name, &pretty_name, |tcx, _| {
            tcx.codegen_variant_struct_fields(case, subst, variant, initial_offset)
        })
    }

    fn codegen_vector(&mut self, ty: Ty<'tcx>) -> Type {
        let layout = &self.layout_of(ty).layout.abi();
        debug! {"handling simd with layout {:?}", layout};

        let (element, size) = match layout {
            Vector { element, count } => (element, count),
            _ => unreachable!(),
        };

        let prim_type = element.primitive();
        let rust_type = self.codegen_prim_typ(prim_type);
        let cbmc_type = self.codegen_ty(rust_type);

        Type::vector(cbmc_type, *size)
    }

    /// the function type of the current instance
    pub fn fn_typ(&mut self, instance: InstanceStable, body: &Body) -> Type {
        let fn_abi = instance.fn_abi().unwrap();
        let params: Vec<Parameter> = self
            .codegen_args(instance, &fn_abi)
            .filter_map(|(i, arg_abi)| {
                let ty = arg_abi.ty;
                debug!(?i, ?arg_abi, "fn_typ");
                if arg_abi.mode == PassMode::Ignore {
                    // We ignore zero-sized parameters.
                    // See https://github.com/model-checking/kani/issues/274 for more details.
                    None
                } else {
                    // An arg is the local with index offset by one (return value is always local 0)
                    let lc = i + 1;
                    let mut ident = self.codegen_var_name(&lc);

                    // `spread_arg` indicates that the last argument is tupled
                    // at the LLVM/codegen level, so we need to declare the individual
                    // components as parameters with a special naming convention
                    // so that we can "retuple" them in the function prelude.
                    // See: compiler/rustc_codegen_llvm/src/gotoc/mod.rs:codegen_function_prelude
                    if let Some(spread) = body.spread_arg() {
                        if lc >= spread {
                            let (name, _) = self.codegen_spread_arg_name(&lc);
                            ident = name;
                        }
                    }
                    Some(
                        self.codegen_ty_stable(ty)
                            .as_parameter(Some(ident.clone().into()), Some(ident.into())),
                    )
                }
            })
            .collect();

        debug!(?params, ?fn_abi, "function_type");
        let ret_type = self.codegen_ty_stable(fn_abi.ret.ty);
        if fn_abi.c_variadic {
            Type::variadic_code(params, ret_type)
        } else {
            Type::code(params, ret_type)
        }
    }

    /// Generate code for a valid object-safe trait receiver type.
    ///
    /// Note that all these types only contain the data pointer and ZST fields. Thus, we generate
    /// the non-ZST branch manually. In some cases, this method is called from inside
    /// `codegen_ty(arg_ty)` so we don't have information about the final type.
    fn codegen_trait_receiver(&mut self, arg_ty: Ty<'tcx>) -> Type {
        // Collect structs that need to be modified
        // Collect the non-ZST fields until we find a fat pointer.
        let mut data_path = vec![arg_ty];
        data_path.extend(self.receiver_data_path(arg_ty).map(|(_, typ)| typ));

        trace!(?arg_ty, ?data_path, "codegen_trait_receiver");
        let orig_pointer_ty = data_path.pop().unwrap();
        assert!(self.is_vtable_fat_pointer(orig_pointer_ty));

        // Traverse type and replace pointer type.
        let ptr_type = self.codegen_trait_data_pointer(pointee_type(orig_pointer_ty).unwrap());
        data_path.iter().rev().fold(ptr_type, |last_type, curr| {
            // Codegen the type replacing the non-zst field.
            let new_name = self.ty_mangled_name(*curr).to_string() + "::WithDataPtr";
            let new_pretty_name = format!("{}::WithDataPtr", self.ty_pretty_name(*curr));
            if let ty::Adt(adt_def, adt_args) = curr.kind() {
                let fields = &adt_def.variants().get(VariantIdx::from_u32(0)).unwrap().fields;
                self.ensure_struct(new_name, new_pretty_name, |ctx, s_name| {
                    let fields_shape = ctx.layout_of(*curr).layout.fields();
                    let components = fields_shape
                        .index_by_increasing_offset()
                        .map(|idx| {
                            let idx = idx.into();
                            let name = fields[idx].name.to_string().intern();
                            let field_ty = fields[idx].ty(ctx.tcx, adt_args);
                            let typ = if !ctx.is_zst(field_ty) {
                                last_type.clone()
                            } else {
                                ctx.codegen_ty(field_ty)
                            };
                            DatatypeComponent::Field { name, typ }
                        })
                        .collect();
                    trace!(?data_path, ?curr, ?s_name, ?components, "codegen_trait_receiver");
                    components
                })
            } else {
                unreachable!("Expected structs only {:?}", curr);
            }
        })
    }
}

/// Use maps instead of lists to manage mir struct components.
impl<'tcx> GotocCtx<'tcx> {
    /// Extract a trait type from a `Struct<dyn T>`.
    /// Note that `T` must be the last element of the struct.
    /// This also handles nested cases: `Struct<Struct<dyn T>>` returns `dyn T`
    pub fn extract_trait_type(&self, struct_type: Ty<'tcx>) -> Option<Ty<'tcx>> {
        if !self.use_vtable_fat_pointer(struct_type) {
            warn!(got=?struct_type, "Expected trait type or a DST struct with a trait element.");
            return None;
        }

        let mut typ = struct_type;
        while let ty::Adt(adt_def, adt_args) = typ.kind() {
            assert_eq!(adt_def.variants().len(), 1, "Expected a single-variant ADT. Found {typ:?}");
            let fields = &adt_def.variants().get(VariantIdx::from_u32(0)).unwrap().fields;
            let last_field = fields.last_index().expect("Trait should be the last element.");
            typ = fields[last_field].ty(self.tcx, adt_args);
        }
        if typ.is_trait() { Some(typ) } else { None }
    }

    /// This function provides an iterator that traverses the data path of a receiver type. I.e.:
    /// the path that leads to the data pointer.
    ///
    /// E.g.: For `Rc<dyn T>` where the Rc definition is:
    /// ```
    /// pub struct Rc<T: ?Sized> {
    ///    ptr: NonNull<RcBox<T>>,
    ///    phantom: PhantomData<RcBox<T>>,
    /// }
    ///
    /// pub struct NonNull<T: ?Sized> {
    ///    pointer: *const T,
    /// }
    /// ```
    ///
    /// The behavior will be:
    /// ```text
    /// let it = self.receiver_data_path(rc_typ);
    /// assert_eq!(it.next(), Some((String::from("ptr"), non_null_typ);
    /// assert_eq!(it.next(), Some((String::from("pointer"), raw_ptr_typ);
    /// assert_eq!(it.next(), None);
    /// ```
    ///
    /// Pre-condition: The argument must be a valid receiver for dispatchable trait functions.
    /// See <https://doc.rust-lang.org/reference/items/traits.html#object-safety> for more details.
    pub fn receiver_data_path<'a>(
        &'a self,
        typ: Ty<'tcx>,
    ) -> impl Iterator<Item = (String, Ty<'tcx>)> + 'a {
        struct ReceiverIter<'tcx, 'a> {
            pub curr: Ty<'tcx>,
            pub ctx: &'a GotocCtx<'tcx>,
        }

        impl<'tcx> Iterator for ReceiverIter<'tcx, '_> {
            type Item = (String, Ty<'tcx>);

            fn next(&mut self) -> Option<Self::Item> {
                if let ty::Adt(adt_def, adt_args) = self.curr.kind() {
                    assert_eq!(
                        adt_def.variants().len(),
                        1,
                        "Expected a single-variant ADT. Found {:?}",
                        self.curr
                    );
                    let ctx = self.ctx;
                    let fields = &adt_def.variants().get(VariantIdx::from_u32(0)).unwrap().fields;
                    let mut non_zsts = fields
                        .iter()
                        .filter(|field| !ctx.is_zst(field.ty(ctx.tcx, adt_args)))
                        .map(|non_zst| (non_zst.name.to_string(), non_zst.ty(ctx.tcx, adt_args)));
                    let (name, next) = non_zsts.next().expect("Expected one non-zst field.");
                    self.curr = next;
                    assert!(non_zsts.next().is_none(), "Expected only one non-zst field.");
                    Some((name, self.curr))
                } else {
                    None
                }
            }
        }

        ReceiverIter { ctx: self, curr: typ }
    }

    /// Allow us to retrieve the instance arguments in a consistent way.
    /// There are two corner cases that we currently handle:
    /// 1. In some cases, an argument can be ignored (e.g.: ZST arguments in regular Rust calls).
    /// 2. We currently don't support `track_caller`, so we ignore the extra argument that is added to support that.
    ///    Tracked here: <https://github.com/model-checking/kani/issues/374>
    pub fn codegen_args<'a>(
        &self,
        instance: InstanceStable,
        fn_abi: &'a FnAbi,
    ) -> impl Iterator<Item = (usize, &'a ArgAbi)> {
        let requires_caller_location = self.requires_caller_location(instance);
        let num_args = fn_abi.args.len();
        fn_abi.args.iter().enumerate().filter(move |(idx, arg_abi)| {
            arg_abi.mode != PassMode::Ignore && !(requires_caller_location && idx + 1 == num_args)
        })
    }
}

/// Return the datatype components for fields are present in every vtable struct.
///
/// We follow the order from the `TyCtxt::::COMMON_VTABLE_ENTRIES`.
fn common_vtable_fields(drop_in_place: Type) -> Vec<DatatypeComponent> {
    let fields: Vec<DatatypeComponent> = TyCtxt::COMMON_VTABLE_ENTRIES
        .iter()
        .map(|entry| match entry {
            VtblEntry::MetadataDropInPlace => {
                DatatypeComponent::field(VTABLE_DROP_FIELD, drop_in_place.clone())
            }
            VtblEntry::MetadataSize => DatatypeComponent::field(VTABLE_SIZE_FIELD, Type::size_t()),
            VtblEntry::MetadataAlign => {
                DatatypeComponent::field(VTABLE_ALIGN_FIELD, Type::size_t())
            }
            VtblEntry::Vacant | VtblEntry::Method(_) | VtblEntry::TraitVPtr(_) => {
                unimplemented!("Entry shouldn't be common: {:?}", entry)
            }
        })
        .collect();
    assert_eq!(fields.len(), 3, "We expect only three common fields for every vtable.");
    fields
}

/// If given type is a Ref / Raw ref, return the pointee type.
pub fn pointee_type(mir_type: Ty) -> Option<Ty> {
    match mir_type.kind() {
        ty::Ref(_, pointee_type, _) => Some(*pointee_type),
        ty::RawPtr(pointee_type, _) => Some(*pointee_type),
        _ => None,
    }
}

/// Extracts the pointee type if the given mir type is either a known smart pointer (Box, Rc, ..)
/// or a regular pointer.
pub fn std_pointee_type(mir_type: Ty) -> Option<Ty> {
    mir_type.builtin_deref(true)
}

/// This is a place holder function that should normalize the given type.
///
/// TODO: We should normalize the type projection here. For more details, see
/// <https://github.com/model-checking/kani/issues/752>
fn normalize_type(ty: Ty) -> Ty {
    ty
}

impl<'tcx> GotocCtx<'tcx> {
    /// A pointer to the mir type should be a thin pointer.
    /// Use thin pointer if the type is sized or if the resulting pointer has no metadata.
    /// Note: Foreign items are unsized but it codegen as a thin pointer since there is no
    /// metadata associated with it.
    pub fn use_thin_pointer(&self, mir_type: Ty<'tcx>) -> bool {
        // ptr_metadata_ty is not defined on all types, the projection of an associated type
        let metadata = mir_type.ptr_metadata_ty_or_tail(self.tcx, normalize_type);
        !self.is_unsized(mir_type)
            || metadata.is_err()
            || (metadata.unwrap() == self.tcx.types.unit)
    }

    /// We use fat pointer if not thin pointer.
    pub fn use_fat_pointer(&self, mir_type: Ty<'tcx>) -> bool {
        !self.use_thin_pointer(mir_type)
    }

    /// A pointer to the mir type should be a slice fat pointer.
    /// We use a slice fat pointer if the metadata is the slice length (type usize).
    pub fn use_slice_fat_pointer(&self, mir_type: Ty<'tcx>) -> bool {
        let metadata = mir_type.ptr_metadata_ty(self.tcx, normalize_type);
        metadata == self.tcx.types.usize
    }
    /// A pointer to the mir type should be a vtable fat pointer.
    /// We use a vtable fat pointer if this is a fat pointer to anything that is not a slice ptr.
    /// I.e.: The metadata is not length (type usize).
    pub fn use_vtable_fat_pointer(&self, mir_type: Ty<'tcx>) -> bool {
        let metadata = mir_type.ptr_metadata_ty(self.tcx, normalize_type);
        metadata != self.tcx.types.unit && metadata != self.tcx.types.usize
    }

    /// Does the current mir represent a fat pointer (Raw pointer or ref)
    /// TODO: Should we use `std_pointee_type` here?
    /// <https://github.com/model-checking/kani/issues/1529>
    pub fn is_fat_pointer(&self, pointer_ty: Ty<'tcx>) -> bool {
        pointee_type(pointer_ty).map_or(false, |pointee_ty| self.use_fat_pointer(pointee_ty))
    }

    /// Check if the mir type already is a vtable fat pointer.
    pub fn is_vtable_fat_pointer(&self, mir_type: Ty<'tcx>) -> bool {
        pointee_type(mir_type).map_or(false, |pointee_ty| self.use_vtable_fat_pointer(pointee_ty))
    }
}
