// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! responsible for handling codegening places.
//!
//! a place is an expression of specifying a location in memory, like a left value. check the cases
//! in [GotocCtx::codegen_place] below.

use super::typ::TypeExt;
use crate::codegen_cprover_gotoc::codegen::ty_stable::{pointee_type, StableConverter};
use crate::codegen_cprover_gotoc::codegen::typ::{
    pointee_type as pointee_type_internal, std_pointee_type,
};
use crate::codegen_cprover_gotoc::utils::{dynamic_fat_ptr, slice_fat_ptr};
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::unwrap_or_return_codegen_unimplemented;
use cbmc::goto_program::{Expr, Location, Type};
use rustc_middle::mir::{Local as LocalInternal, Place as PlaceInternal};
use rustc_middle::ty::layout::LayoutOf;
use rustc_smir::rustc_internal;
use rustc_target::abi::{TagEncoding, Variants};
use stable_mir::mir::{FieldIdx, Local, Mutability, Place, ProjectionElem};
use stable_mir::ty::{RigidTy, Ty, TyKind, VariantDef, VariantIdx};
use tracing::{debug, trace, warn};

/// A projection in Kani can either be to a type (the normal case),
/// or a variant in the case of a downcast.
#[derive(Copy, Clone, Debug)]
pub enum TypeOrVariant {
    Type(Ty),
    Variant(VariantDef),
    CoroutineVariant(VariantIdx),
}

/// A struct for storing the data for passing to `codegen_unimplemented`
#[derive(Debug)]
pub struct UnimplementedData {
    /// The specific operation that is not supported
    pub operation: String,
    /// URL for issue on Kani github page
    pub bug_url: String,
    /// The resulting goto type of the operation
    pub goto_type: Type,
    /// Location of operation
    pub loc: Location,
}

impl UnimplementedData {
    pub fn new(operation: &str, bug_url: &str, goto_type: Type, loc: Location) -> Self {
        UnimplementedData {
            operation: operation.to_string(),
            bug_url: bug_url.to_string(),
            goto_type,
            loc,
        }
    }
}

/// Relevent information about a projected place (i.e. an lvalue).
#[derive(Debug)]
pub struct ProjectedPlace {
    /// The goto expression that represents the lvalue
    pub goto_expr: Expr,
    /// The MIR type of that expression. Normally a type, but can be a variant following a downcast.
    pub mir_typ_or_variant: TypeOrVariant,
    /// If a fat pointer was traversed during the projection, it is stored here.
    /// This is useful if we need to use any of its fields, for e.g. to generate a rvalue ref
    /// or to implement the `length` operation.
    pub fat_ptr_goto_expr: Option<Expr>,
    /// The MIR type of the visited fat pointer, if one was traversed during the projection.
    pub fat_ptr_mir_typ: Option<Ty>,
}

/// Getters
#[allow(dead_code)]
impl ProjectedPlace {
    pub fn goto_expr(&self) -> &Expr {
        &self.goto_expr
    }

    pub fn mir_typ_or_variant(&self) -> &TypeOrVariant {
        &self.mir_typ_or_variant
    }

    pub fn mir_typ(&self) -> Ty {
        self.mir_typ_or_variant.expect_type()
    }

    pub fn fat_ptr_goto_expr(&self) -> &Option<Expr> {
        &self.fat_ptr_goto_expr
    }

    pub fn fat_ptr_mir_typ(&self) -> &Option<Ty> {
        &self.fat_ptr_mir_typ
    }
}

/// Constructor
impl ProjectedPlace {
    fn check_expr_typ_mismatch(
        expr: &Expr,
        typ: &TypeOrVariant,
        ctx: &mut GotocCtx,
    ) -> Option<(Type, Type)> {
        match typ {
            TypeOrVariant::Type(t) => {
                let expr_ty = expr.typ().clone();
                let type_from_mir = ctx.codegen_ty_stable(*t);
                if expr_ty != type_from_mir {
                    match t.kind() {
                        // Slice references (`&[T]`) store raw pointers to the element type `T`
                        // due to pointer decay. They are fat pointers with the following repr:
                        // SliceRef { data: *T, len: usize }.
                        // In those cases, the projection will yield a pointer type.
                        TyKind::RigidTy(RigidTy::Slice(..)) | TyKind::RigidTy(RigidTy::Str)
                            if expr_ty.is_pointer()
                                && expr_ty.base_type() == type_from_mir.base_type() =>
                        {
                            None
                        }
                        // TODO: Do we really need this?
                        // https://github.com/model-checking/kani/issues/1092
                        TyKind::RigidTy(RigidTy::Dynamic(..))
                            if expr_ty.is_pointer()
                                && *expr_ty.base_type().unwrap() == type_from_mir =>
                        {
                            None
                        }
                        _ => Some((expr_ty, type_from_mir)),
                    }
                } else {
                    None
                }
            }
            // TODO: handle Variant https://github.com/model-checking/kani/issues/448
            TypeOrVariant::Variant(_) | TypeOrVariant::CoroutineVariant(_) => None,
        }
    }

    fn check_fat_ptr_typ(
        fat_ptr: &Option<Expr>,
        fat_ptr_typ: &Option<Ty>,
        ctx: &mut GotocCtx,
    ) -> bool {
        if let Some(fat_ptr) = fat_ptr {
            fat_ptr.typ().is_rust_fat_ptr(&ctx.symbol_table)
                && fat_ptr.typ() == &ctx.codegen_ty_stable(fat_ptr_typ.unwrap())
        } else {
            true
        }
    }

    pub fn try_from_ty(
        goto_expr: Expr,
        ty: Ty,
        ctx: &mut GotocCtx,
    ) -> Result<Self, UnimplementedData> {
        Self::try_new(goto_expr, TypeOrVariant::Type(ty), None, None, ctx)
    }

    pub fn try_new(
        goto_expr: Expr,
        mir_typ_or_variant: TypeOrVariant,
        fat_ptr_goto_expr: Option<Expr>,
        fat_ptr_mir_typ: Option<Ty>,
        ctx: &mut GotocCtx,
    ) -> Result<Self, UnimplementedData> {
        if let Some(fat_ptr) = &fat_ptr_goto_expr {
            assert!(
                fat_ptr.typ().is_rust_fat_ptr(&ctx.symbol_table),
                "Expected fat pointer, got {:?} in function {}",
                fat_ptr.typ(),
                ctx.current_fn().readable_name()
            );
        }
        if let Some((expr_ty, ty_from_mir)) =
            Self::check_expr_typ_mismatch(&goto_expr, &mir_typ_or_variant, ctx)
        {
            let msg = format!(
                "Unexpected type mismatch in projection:\n{goto_expr:?}\nExpr type\n{expr_ty:?}\nType from MIR\n{ty_from_mir:?}"
            );
            warn!("{}", msg);
            // TODO: there's an expr type mismatch with the rust 2022-11-20 toolchain
            // for simd:
            // https://github.com/model-checking/kani/issues/1926
            // Disabling it for this specific case.
            if !(expr_ty.is_integer() && ty_from_mir.is_struct_tag()) {
                debug_assert!(false, "{}", msg);
            }
            return Err(UnimplementedData::new(
                "Projection mismatch",
                "https://github.com/model-checking/kani/issues/277",
                ty_from_mir,
                *goto_expr.location(),
            ));
        }

        assert!(
            Self::check_fat_ptr_typ(&fat_ptr_goto_expr, &fat_ptr_mir_typ, ctx),
            "\n{:?}\n{:?}",
            &fat_ptr_goto_expr,
            &fat_ptr_mir_typ
        );
        Ok(ProjectedPlace { goto_expr, mir_typ_or_variant, fat_ptr_goto_expr, fat_ptr_mir_typ })
    }
}

impl TypeOrVariant {
    pub fn expect_type(&self) -> Ty {
        match self {
            TypeOrVariant::Type(t) => *t,
            TypeOrVariant::Variant(v) => panic!("expect a type but variant is found: {v:?}"),
            TypeOrVariant::CoroutineVariant(v) => {
                panic!("expect a type but coroutine variant is found: {v:?}")
            }
        }
    }

    #[allow(dead_code)]
    pub fn expect_variant(&self) -> &VariantDef {
        match self {
            TypeOrVariant::Type(t) => panic!("expect a variant but type is found: {t:?}"),
            TypeOrVariant::Variant(v) => v,
            TypeOrVariant::CoroutineVariant(v) => {
                panic!("expect a variant but coroutine variant found {v:?}")
            }
        }
    }
}

impl<'tcx> GotocCtx<'tcx> {
    /// Codegen field access for types that allow direct field projection.
    ///
    /// I.e.: Algebraic data types, closures, and coroutines.
    ///
    /// Other composite types such as array only support index projection.
    fn codegen_field(
        &mut self,
        parent_expr: Expr,
        parent_ty_or_var: TypeOrVariant,
        field_idx: FieldIdx,
        field_ty_or_var: TypeOrVariant,
    ) -> Result<Expr, UnimplementedData> {
        match parent_ty_or_var {
            TypeOrVariant::Type(parent_ty) => {
                match parent_ty.kind() {
                    TyKind::Alias(..)
                    | TyKind::RigidTy(RigidTy::Bool)
                    | TyKind::RigidTy(RigidTy::Char)
                    | TyKind::RigidTy(RigidTy::Int(_))
                    | TyKind::RigidTy(RigidTy::Uint(_))
                    | TyKind::RigidTy(RigidTy::Float(_))
                    | TyKind::RigidTy(RigidTy::FnPtr(_))
                    | TyKind::RigidTy(RigidTy::Never)
                    | TyKind::RigidTy(RigidTy::FnDef(..))
                    | TyKind::RigidTy(RigidTy::CoroutineWitness(..))
                    | TyKind::RigidTy(RigidTy::Foreign(..))
                    | TyKind::RigidTy(RigidTy::Dynamic(..))
                    | TyKind::Bound(..)
                    | TyKind::Param(..) => {
                        unreachable!("type {parent_ty:?} does not have a field")
                    }
                    TyKind::RigidTy(RigidTy::Tuple(_)) => {
                        Ok(parent_expr.member(Self::tuple_fld_name(field_idx), &self.symbol_table))
                    }
                    TyKind::RigidTy(RigidTy::Adt(def, _))
                        if rustc_internal::internal(def).repr().simd() =>
                    {
                        Ok(self.codegen_simd_field(
                            parent_expr,
                            field_idx,
                            field_ty_or_var.expect_type(),
                        ))
                    }
                    // if we fall here, then we are handling either a struct or a union
                    TyKind::RigidTy(RigidTy::Adt(def, _)) => {
                        let fields = def.variants_iter().next().unwrap().fields();
                        let field = &fields[field_idx];
                        Ok(parent_expr.member(field.name.to_string(), &self.symbol_table))
                    }
                    TyKind::RigidTy(RigidTy::Closure(..)) => {
                        Ok(parent_expr.member(field_idx.to_string(), &self.symbol_table))
                    }
                    TyKind::RigidTy(RigidTy::Coroutine(..)) => {
                        let field_name = self.coroutine_field_name(field_idx);
                        Ok(parent_expr
                            .member("direct_fields", &self.symbol_table)
                            .member(field_name, &self.symbol_table))
                    }
                    TyKind::RigidTy(RigidTy::Str)
                    | TyKind::RigidTy(RigidTy::Array(_, _))
                    | TyKind::RigidTy(RigidTy::Slice(_))
                    | TyKind::RigidTy(RigidTy::RawPtr(..))
                    | TyKind::RigidTy(RigidTy::Ref(_, _, _)) => {
                        unreachable!(
                            "element of {parent_ty:?} is not accessed via field projection"
                        )
                    }
                }
            }
            // if we fall here, then we are handling an enum
            TypeOrVariant::Variant(parent_var) => {
                let fields = parent_var.fields();
                let field = &fields[field_idx];
                Ok(parent_expr.member(field.name.to_string(), &self.symbol_table))
            }
            TypeOrVariant::CoroutineVariant(_var_idx) => {
                let field_name = self.coroutine_field_name(field_idx);
                Ok(parent_expr.member(field_name, &self.symbol_table))
            }
        }
    }

    /// This is a SIMD vector, which has 2 possible internal representations:
    /// 1- Multi-field representation (original and currently deprecated)
    ///    In this case, a field is one lane (i.e.: one element)
    ///    Example:
    /// ```ignore
    ///    pub struct i64x2(i64, i64);
    ///    fn main() {
    ///      let v = i64x2(1, 2);
    ///      assert!(v.0 == 1); // refers to the first i64
    ///      assert!(v.1 == 2);
    ///    }
    /// ```
    /// 2- Array-based representation
    ///    In this case, the projection refers to the entire array.
    /// ```ignore
    ///    pub struct i64x2([i64; 2]);
    ///    fn main() {
    ///      let v = i64x2([1, 2]);
    ///      assert!(v.0 == [1, 2]); // refers to the entire array
    ///    }
    /// ```
    /// * Note that projection inside SIMD structs may eventually become illegal.
    /// See <https://github.com/rust-lang/stdarch/pull/1422#discussion_r1176415609> thread.
    ///
    /// Since the goto representation for both is the same, we use the expected type to decide
    /// what to return.
    fn codegen_simd_field(&mut self, parent_expr: Expr, field_idx: FieldIdx, field_ty: Ty) -> Expr {
        if matches!(field_ty.kind(), TyKind::RigidTy(RigidTy::Array { .. })) {
            // Array based
            assert_eq!(field_idx, 0);
            let field_typ = self.codegen_ty_stable(field_ty);
            parent_expr.reinterpret_cast(field_typ)
        } else {
            // Return the given field.
            let index_expr = Expr::int_constant(field_idx, Type::size_t());
            parent_expr.index_array(index_expr)
        }
    }

    /// If a local is a function definition, ignore the local variable name and
    /// generate a function call based on the def id.
    ///
    /// Note that this is finicky. A local might be a function definition, a
    /// pointer to one, or a boxed pointer to one. For example, the
    /// auto-generated code for Fn::call_once uses a local FnDef to call the
    /// wrapped function, while the auto-generated code for Fn::call and
    /// Fn::call_mut both use pointers to a FnDef. In these cases, we need to
    /// generate an expression that references the existing FnDef rather than
    /// a named variable.
    ///
    /// Recursively finds the actual FnDef from a pointer or box.
    fn codegen_local_fndef(&mut self, ty: Ty) -> Option<Expr> {
        match ty.kind() {
            // A local that is itself a FnDef, like Fn::call_once
            TyKind::RigidTy(RigidTy::FnDef(def, args)) => {
                Some(self.codegen_fndef(def, &args, None))
            }
            // A local can be pointer to a FnDef, like Fn::call and Fn::call_mut
            TyKind::RigidTy(RigidTy::RawPtr(inner, _)) => self
                .codegen_local_fndef(inner)
                .map(|f| if f.can_take_address_of() { f.address_of() } else { f }),
            // A local can be a boxed function pointer
            TyKind::RigidTy(RigidTy::Adt(def, args)) if def.is_box() => {
                let boxed_ty = self.codegen_ty_stable(ty);
                // The type of `T` for `Box<T>` can be derived from the first definition args.
                let inner_ty = args.0[0].ty().unwrap();
                self.codegen_local_fndef(*inner_ty)
                    .map(|f| self.box_value(f.address_of(), boxed_ty))
            }
            _ => None,
        }
    }

    /// Codegen for a local
    fn codegen_local(&mut self, l: Local) -> Expr {
        let local_ty = self.local_ty_stable(l);
        // Check if the local is a function definition (see comment above)
        if let Some(fn_def) = self.codegen_local_fndef(local_ty) {
            return fn_def;
        }

        // Otherwise, simply look up the local by the var name.
        let vname = self.codegen_var_name(&LocalInternal::from(l));
        Expr::symbol_expression(vname, self.codegen_ty_stable(local_ty))
    }

    /// A projection is an operation that translates an lvalue to another lvalue.
    /// E.g. dereference, follow a field, etc.
    /// This function codegens a single step of a projection.
    /// `before` is the expression "before" this projection is applied;
    /// the return value is the expression after.
    fn codegen_projection(
        &mut self,
        before: Result<ProjectedPlace, UnimplementedData>,
        proj: &ProjectionElem,
    ) -> Result<ProjectedPlace, UnimplementedData> {
        let before = before?;
        trace!(?before, ?proj, "codegen_projection");
        match proj {
            ProjectionElem::Deref => {
                let base_type = before.mir_typ();
                let inner_goto_expr = if is_box(base_type) {
                    self.deref_box(before.goto_expr)
                } else {
                    before.goto_expr
                };

                let inner_mir_typ_internal =
                    std_pointee_type(rustc_internal::internal(base_type)).unwrap();
                let inner_mir_typ = rustc_internal::stable(inner_mir_typ_internal);
                let (fat_ptr_mir_typ, fat_ptr_goto_expr) = if self
                    .use_thin_pointer(inner_mir_typ_internal)
                {
                    (before.fat_ptr_mir_typ, before.fat_ptr_goto_expr)
                } else {
                    (Some(before.mir_typ_or_variant.expect_type()), Some(inner_goto_expr.clone()))
                };

                // Check that we have a valid trait or slice fat pointer
                if let Some(fat_ptr) = fat_ptr_goto_expr.clone() {
                    assert!(
                        fat_ptr.typ().is_rust_trait_fat_ptr(&self.symbol_table)
                            || fat_ptr.typ().is_rust_slice_fat_ptr(&self.symbol_table),
                        "Unexpected type: {:?} -- {:?}",
                        fat_ptr.typ(),
                        pointee_type(fat_ptr_mir_typ.unwrap()).unwrap().kind(),
                    );
                    assert!(
                        self.use_fat_pointer(rustc_internal::internal(
                            pointee_type(fat_ptr_mir_typ.unwrap()).unwrap()
                        )),
                        "Unexpected type: {:?} -- {:?}",
                        fat_ptr.typ(),
                        fat_ptr_mir_typ,
                    );
                };

                let expr = match inner_mir_typ.kind() {
                    TyKind::RigidTy(RigidTy::Slice(_))
                    | TyKind::RigidTy(RigidTy::Str)
                    | TyKind::RigidTy(RigidTy::Dynamic(..)) => {
                        inner_goto_expr.member("data", &self.symbol_table)
                    }
                    TyKind::RigidTy(RigidTy::Adt(..))
                        if self.is_unsized(inner_mir_typ_internal) =>
                    {
                        // in tests/kani/Strings/os_str_reduced.rs, we see
                        // ```
                        //  p.projection = [
                        //     Deref,
                        //     Field(
                        //         field[0],
                        //         [u8],
                        //     ),
                        // ]
                        // ```
                        // This implies that the result of a deref on an ADT fat pointer
                        // should be the ADT itself. So we need the `.dereference()` here.
                        // Note that this causes problems in `codegen_rvalue_ref()`.
                        // See the comment there for more details.
                        inner_goto_expr
                            .member("data", &self.symbol_table)
                            // In the case of a vtable fat pointer, this data member is a void pointer,
                            // so ensure the pointer has the correct type before dereferencing it.
                            .cast_to(self.codegen_ty_stable(inner_mir_typ).to_pointer())
                            .dereference()
                    }
                    _ => inner_goto_expr.dereference(),
                };
                let typ = TypeOrVariant::Type(inner_mir_typ);
                ProjectedPlace::try_new(expr, typ, fat_ptr_goto_expr, fat_ptr_mir_typ, self)
            }
            ProjectionElem::Field(idx, ty) => {
                let typ = TypeOrVariant::Type(*ty);
                let expr =
                    self.codegen_field(before.goto_expr, before.mir_typ_or_variant, *idx, typ)?;
                ProjectedPlace::try_new(
                    expr,
                    typ,
                    before.fat_ptr_goto_expr,
                    before.fat_ptr_mir_typ,
                    self,
                )
            }
            ProjectionElem::Index(i) => {
                let base_type = before.mir_typ();
                let idxe = self.codegen_local(*i);
                let typ = match base_type.kind() {
                    TyKind::RigidTy(RigidTy::Array(elemt, _))
                    | TyKind::RigidTy(RigidTy::Slice(elemt)) => TypeOrVariant::Type(elemt),
                    _ => unreachable!("must index an array"),
                };
                let expr = match base_type.kind() {
                    TyKind::RigidTy(RigidTy::Array(..)) => {
                        self.codegen_idx_array(before.goto_expr, idxe)
                    }
                    TyKind::RigidTy(RigidTy::Slice(..)) => before.goto_expr.index(idxe),
                    _ => unreachable!("must index an array"),
                };
                ProjectedPlace::try_new(
                    expr,
                    typ,
                    before.fat_ptr_goto_expr,
                    before.fat_ptr_mir_typ,
                    self,
                )
            }
            ProjectionElem::ConstantIndex { offset, min_length, from_end } => {
                self.codegen_constant_index(before, *offset, *min_length, *from_end)
            }
            // Best effort to codegen subslice projection.
            // Full support to be added in
            // https://github.com/model-checking/kani/issues/707
            ProjectionElem::Subslice { from, to, from_end } => {
                // https://rust-lang.github.io/rfcs/2359-subslice-pattern-syntax.html
                match before.mir_typ().kind() {
                    TyKind::RigidTy(RigidTy::Array(ty, len)) => {
                        let len = len.eval_target_usize().unwrap();
                        let subarray_len = if *from_end {
                            // `to` counts from the end of the array
                            len - to - from
                        } else {
                            to - from
                        };
                        let typ = Ty::try_new_array(ty, subarray_len).unwrap();
                        let goto_typ = self.codegen_ty_stable(typ);
                        // unimplemented
                        Err(UnimplementedData::new(
                            "Sub-array binding",
                            "https://github.com/model-checking/kani/issues/707",
                            goto_typ,
                            *before.goto_expr.location(),
                        ))
                    }
                    TyKind::RigidTy(RigidTy::Slice(_)) => {
                        let len = if *from_end {
                            let olen = before
                                .fat_ptr_goto_expr
                                .clone()
                                .unwrap()
                                .member("len", &self.symbol_table);
                            let sum = Expr::int_constant(to + from, Type::size_t());
                            olen.sub(sum) // olen - (to + from) = olen - to - from
                        } else {
                            Expr::int_constant(to - from, Type::size_t())
                        };
                        let typ = before.mir_typ();
                        let ptr_typ = Ty::new_ptr(typ, Mutability::Not);
                        let goto_type = self.codegen_ty_stable(ptr_typ);

                        let index = Expr::int_constant(*from, Type::ssize_t());
                        let from_elem = before.goto_expr.index(index);
                        let data = from_elem.address_of();
                        let fat_ptr = slice_fat_ptr(goto_type, data, len, &self.symbol_table);
                        ProjectedPlace::try_new(
                            fat_ptr.clone(),
                            TypeOrVariant::Type(ptr_typ),
                            Some(fat_ptr),
                            Some(ptr_typ),
                            self,
                        )
                    }
                    _ => unreachable!("must be array or slice"),
                }
            }
            ProjectionElem::Downcast(idx) => {
                // downcast converts a variable of an enum type to one of its discriminated cases
                let ty = before.mir_typ();
                let ty_kind = ty.kind();
                let (case_name, type_or_variant) = match &ty_kind {
                    TyKind::RigidTy(RigidTy::Adt(def, _)) => {
                        let variant = def.variant(*idx).unwrap();
                        (variant.name().into(), TypeOrVariant::Variant(variant))
                    }
                    TyKind::RigidTy(RigidTy::Coroutine(..)) => {
                        let idx_internal = rustc_internal::internal(idx);
                        (
                            self.coroutine_variant_name(idx_internal),
                            TypeOrVariant::CoroutineVariant(*idx),
                        )
                    }
                    _ => unreachable!(
                        "cannot downcast {:?} to a variant (only enums and coroutines can)",
                        &ty.kind()
                    ),
                };
                let layout = self.layout_of(rustc_internal::internal(ty));
                let expr = match &layout.variants {
                    Variants::Single { .. } => before.goto_expr,
                    Variants::Multiple { tag_encoding, .. } => match tag_encoding {
                        TagEncoding::Direct => {
                            let cases = if is_coroutine(ty_kind) {
                                before.goto_expr
                            } else {
                                before.goto_expr.member("cases", &self.symbol_table)
                            };
                            cases.member(case_name, &self.symbol_table)
                        }
                        TagEncoding::Niche { .. } => {
                            before.goto_expr.member(case_name, &self.symbol_table)
                        }
                    },
                };
                ProjectedPlace::try_new(
                    expr,
                    type_or_variant,
                    before.fat_ptr_goto_expr,
                    before.fat_ptr_mir_typ,
                    self,
                )
            }
            ProjectionElem::OpaqueCast(ty) | ProjectionElem::Subtype(ty) => {
                ProjectedPlace::try_new(
                    before.goto_expr.cast_to(self.codegen_ty_stable(*ty)),
                    TypeOrVariant::Type(*ty),
                    before.fat_ptr_goto_expr,
                    before.fat_ptr_mir_typ,
                    self,
                )
            }
        }
    }

    /// Codegen the reference to a given place.
    /// We currently have a somewhat weird way of handling ZST.
    /// - For `*(&T)` where `T: Unsized`, the projection's `goto_expr` is a thin pointer, so we
    ///   build the fat pointer from there.
    /// - For `*(Wrapper<T>)` where `T: Unsized`, the projection's `goto_expr` returns an object,
    ///   and we need to take it's address and build the fat pointer.
    pub fn codegen_place_ref(&mut self, place: &PlaceInternal<'tcx>) -> Expr {
        let place_ty = self.place_ty(place);
        let projection = unwrap_or_return_codegen_unimplemented!(self, self.codegen_place(place));
        if self.use_thin_pointer(place_ty) {
            // Just return the address of the place dereferenced.
            projection.goto_expr.address_of()
        } else if place_ty == pointee_type_internal(self.local_ty(place.local)).unwrap() {
            // Just return the fat pointer if this is a simple &(*local).
            projection.fat_ptr_goto_expr.unwrap()
        } else {
            // Build a new fat pointer to the place dereferenced with the metadata from the
            // original fat pointer.
            let data = projection_data_ptr(&projection);
            let fat_ptr = projection.fat_ptr_goto_expr.unwrap();
            let place_type = self.codegen_ty_ref(place_ty);
            if self.use_vtable_fat_pointer(place_ty) {
                let vtable = fat_ptr.member("vtable", &self.symbol_table);
                dynamic_fat_ptr(place_type, data, vtable, &self.symbol_table)
            } else {
                let len = fat_ptr.member("len", &self.symbol_table);
                slice_fat_ptr(place_type, data, len, &self.symbol_table)
            }
        }
    }

    /// Given a MIR place, generate a CBMC expression that represents it as a CBMC lvalue.
    /// A place is the rust term for an lvalue.
    /// Like in "C", a place can be a "projected": e.g. `*x.foo = bar`
    /// This function follows the MIR projection to get the final useable lvalue.
    /// If it passes through a fat pointer along the way, it stores info about it,
    /// which can be useful in reconstructing fat pointer operations.
    pub fn codegen_place_stable(
        &mut self,
        place: &Place,
    ) -> Result<ProjectedPlace, UnimplementedData> {
        debug!(?place, "codegen_place");
        let initial_expr = self.codegen_local(place.local);
        let initial_typ = TypeOrVariant::Type(self.local_ty_stable(place.local));
        debug!(?initial_typ, ?initial_expr, "codegen_place");
        let initial_projection =
            ProjectedPlace::try_new(initial_expr, initial_typ, None, None, self);
        let result = place
            .projection
            .iter()
            .fold(initial_projection, |accum, proj| self.codegen_projection(accum, proj));
        match result {
            Err(data) => Err(UnimplementedData::new(
                &data.operation,
                &data.bug_url,
                self.codegen_ty_stable(self.place_ty_stable(place)),
                data.loc,
            )),
            _ => result,
        }
    }

    pub fn codegen_place(
        &mut self,
        place: &PlaceInternal<'tcx>,
    ) -> Result<ProjectedPlace, UnimplementedData> {
        self.codegen_place_stable(&StableConverter::convert_place(self, *place))
    }

    /// Given a projection, generate an lvalue that represents the given variant index.
    pub fn codegen_variant_lvalue(
        &mut self,
        initial_projection: ProjectedPlace,
        variant_idx: VariantIdx,
    ) -> ProjectedPlace {
        debug!(?initial_projection, ?variant_idx, "codegen_variant_lvalue");
        let downcast = ProjectionElem::Downcast(variant_idx);
        self.codegen_projection(Ok(initial_projection), &downcast).unwrap()
    }

    // https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.ProjectionElem.html
    // ConstantIndex
    // [−]
    // These indices are generated by slice patterns. Easiest to explain by example:
    // [X, _, .._, _, _] => { offset: 0, min_length: 4, from_end: false },
    // [_, X, .._, _, _] => { offset: 1, min_length: 4, from_end: false },
    // [_, _, .._, X, _] => { offset: 2, min_length: 4, from_end: true },
    // [_, _, .._, _, X] => { offset: 1, min_length: 4, from_end: true },
    fn codegen_constant_index(
        &mut self,
        before: ProjectedPlace,
        offset: u64,
        min_length: u64,
        from_end: bool,
    ) -> Result<ProjectedPlace, UnimplementedData> {
        match before.mir_typ().kind() {
            //TODO, ask on zulip if we can ever have from_end here?
            TyKind::RigidTy(RigidTy::Array(elemt, length)) => {
                let length = length.eval_target_usize().unwrap();
                assert!(length >= min_length);
                let idx = if from_end { length - offset } else { offset };
                let idxe = Expr::int_constant(idx, Type::ssize_t());
                let expr = self.codegen_idx_array(before.goto_expr, idxe);
                let typ = TypeOrVariant::Type(elemt);
                ProjectedPlace::try_new(
                    expr,
                    typ,
                    before.fat_ptr_goto_expr,
                    before.fat_ptr_mir_typ,
                    self,
                )
            }
            TyKind::RigidTy(RigidTy::Slice(elemt)) => {
                let offset_e = Expr::int_constant(offset, Type::size_t());
                //TODO, should we assert min_length? Or is that already handled by the typechecker?
                let idxe = if from_end {
                    let length =
                        before.fat_ptr_goto_expr.clone().unwrap().member("len", &self.symbol_table);
                    length.sub(offset_e)
                } else {
                    offset_e
                };
                let expr = before.goto_expr.plus(idxe).dereference();
                let typ = TypeOrVariant::Type(elemt);
                ProjectedPlace::try_new(
                    expr,
                    typ,
                    before.fat_ptr_goto_expr,
                    before.fat_ptr_mir_typ,
                    self,
                )
            }
            x => unreachable!(
                "Only expected constant index for arrays and slices: also found it for:\n\t{:?}",
                x
            ),
        }
    }

    pub fn codegen_idx_array(&mut self, arr: Expr, idx: Expr) -> Expr {
        arr.index_array(idx)
    }
}

fn is_box(ty: Ty) -> bool {
    matches!(ty.kind(), TyKind::RigidTy(RigidTy::Adt(def, _)) if def.is_box())
}

fn is_coroutine(ty_kind: TyKind) -> bool {
    matches!(ty_kind, TyKind::RigidTy(RigidTy::Coroutine(..)))
}

/// Extract the data pointer from a projection.
/// The return type of the projection is not consistent today, so we need to specialize the
/// behavior in order to get a consistent expression that represents a pointer to the projected
/// data. The cases are:
///  - For `dyn T`, the projection already generates a pointer.
///  - For slices, the projection returns a flexible array.
///  - For structs, like `Wrapper<dyn T>`, the projection returns the object.
fn projection_data_ptr(projection: &ProjectedPlace) -> Expr {
    let proj_expr = projection.goto_expr.clone();
    if proj_expr.typ().is_pointer() {
        proj_expr
    } else if proj_expr.typ().is_array_like() {
        proj_expr.array_to_ptr()
    } else {
        proj_expr.address_of()
    }
}

/// A convenience macro that unwraps a `Result<ProjectPlace<'tcx>,
/// Err<UnimplementedData>` if it is `Ok` and returns an `codegen_unimplemented`
/// expression otherwise.
/// Note that this macro affects the control flow since it calls `return`
#[macro_export]
macro_rules! unwrap_or_return_codegen_unimplemented {
    ($ctx:expr, $pp_result:expr) => {{
        if let Err(err) = $pp_result {
            return $ctx.codegen_unimplemented_expr(
                err.operation.as_str(),
                err.goto_type,
                err.loc,
                err.bug_url.as_str(),
            );
        }
        $pp_result.unwrap()
    }};
}

/// Same as the above macro, but returns a goto program `Stmt` instead
#[macro_export]
macro_rules! unwrap_or_return_codegen_unimplemented_stmt {
    ($ctx:expr, $pp_result:expr) => {{
        if let Err(err) = $pp_result {
            return $ctx.codegen_unimplemented_stmt(
                err.operation.as_str(),
                err.loc,
                err.bug_url.as_str(),
            );
        }
        $pp_result.unwrap()
    }};
}
