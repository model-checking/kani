// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! responsible for handling codegening places.
//!
//! a place is an expression of specifying a location in memory, like a left value. check the cases
//! in [GotocCtx::codegen_place] below.

use super::typ::TypeExt;
use crate::codegen_cprover_gotoc::codegen::typ::{pointee_type, std_pointee_type};
use crate::codegen_cprover_gotoc::utils::{dynamic_fat_ptr, slice_fat_ptr};
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::unwrap_or_return_codegen_unimplemented;
use cbmc::goto_program::{Expr, Location, Type};
use rustc_abi::FieldIdx;
use rustc_hir::Mutability;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::{
    mir::{Local, Place, ProjectionElem},
    ty::{self, Ty, TypeAndMut, VariantDef},
};
use rustc_target::abi::{TagEncoding, VariantIdx, Variants};
use tracing::{debug, trace, warn};

/// A projection in Kani can either be to a type (the normal case),
/// or a variant in the case of a downcast.
#[derive(Copy, Clone, Debug)]
pub enum TypeOrVariant<'tcx> {
    Type(Ty<'tcx>),
    Variant(&'tcx VariantDef),
    GeneratorVariant(VariantIdx),
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
pub struct ProjectedPlace<'tcx> {
    /// The goto expression that represents the lvalue
    pub goto_expr: Expr,
    /// The MIR type of that expression. Normally a type, but can be a variant following a downcast.
    /// Invariant: guaranteed to be monomorphized by the type constructor
    pub mir_typ_or_variant: TypeOrVariant<'tcx>,
    /// If a fat pointer was traversed during the projection, it is stored here.
    /// This is useful if we need to use any of its fields, for e.g. to generate a rvalue ref
    /// or to implement the `length` operation.
    pub fat_ptr_goto_expr: Option<Expr>,
    /// The MIR type of the visited fat pointer, if one was traversed during the projection.
    /// Invariant: guaranteed to be monomorphized by the type constructor
    pub fat_ptr_mir_typ: Option<Ty<'tcx>>,
}

/// Getters
#[allow(dead_code)]
impl<'tcx> ProjectedPlace<'tcx> {
    pub fn goto_expr(&self) -> &Expr {
        &self.goto_expr
    }

    pub fn mir_typ_or_variant(&self) -> &TypeOrVariant<'tcx> {
        &self.mir_typ_or_variant
    }

    pub fn mir_typ(&self) -> Ty<'tcx> {
        self.mir_typ_or_variant.expect_type()
    }

    pub fn fat_ptr_goto_expr(&self) -> &Option<Expr> {
        &self.fat_ptr_goto_expr
    }

    pub fn fat_ptr_mir_typ(&self) -> &Option<Ty<'tcx>> {
        &self.fat_ptr_mir_typ
    }
}

/// Constructor
impl<'tcx> ProjectedPlace<'tcx> {
    fn check_expr_typ_mismatch(
        expr: &Expr,
        typ: &TypeOrVariant<'tcx>,
        ctx: &mut GotocCtx<'tcx>,
    ) -> Option<(Type, Type)> {
        match typ {
            TypeOrVariant::Type(t) => {
                let expr_ty = expr.typ().clone();
                let type_from_mir = ctx.codegen_ty(*t);
                if expr_ty != type_from_mir {
                    match t.kind() {
                        // Slice references (`&[T]`) store raw pointers to the element type `T`
                        // due to pointer decay. They are fat pointers with the following repr:
                        // SliceRef { data: *T, len: usize }.
                        // In those cases, the projection will yield a pointer type.
                        ty::Slice(..) | ty::Str
                            if expr_ty.is_pointer()
                                && expr_ty.base_type() == type_from_mir.base_type() =>
                        {
                            None
                        }
                        // TODO: Do we really need this?
                        // https://github.com/model-checking/kani/issues/1092
                        ty::Dynamic(..)
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
            TypeOrVariant::Variant(_) | TypeOrVariant::GeneratorVariant(_) => None,
        }
    }

    fn check_fat_ptr_typ(
        fat_ptr: &Option<Expr>,
        fat_ptr_typ: &Option<Ty<'tcx>>,
        ctx: &mut GotocCtx<'tcx>,
    ) -> bool {
        if let Some(fat_ptr) = fat_ptr {
            fat_ptr.typ().is_rust_fat_ptr(&ctx.symbol_table)
                && fat_ptr.typ() == &ctx.codegen_ty(fat_ptr_typ.unwrap())
        } else {
            true
        }
    }

    pub fn try_new(
        goto_expr: Expr,
        mir_typ_or_variant: TypeOrVariant<'tcx>,
        fat_ptr_goto_expr: Option<Expr>,
        fat_ptr_mir_typ: Option<Ty<'tcx>>,
        ctx: &mut GotocCtx<'tcx>,
    ) -> Result<Self, UnimplementedData> {
        let mir_typ_or_variant = mir_typ_or_variant.monomorphize(ctx);
        let fat_ptr_mir_typ = fat_ptr_mir_typ.map(|t| ctx.monomorphize(t));
        if let Some(fat_ptr) = &fat_ptr_goto_expr {
            assert!(
                fat_ptr.typ().is_rust_fat_ptr(&ctx.symbol_table),
                "Expected fat pointer, got {:?} in function {}",
                fat_ptr.typ(),
                ctx.current_fn().readable_name()
            );
        }
        // TODO: these assertions fail on a few regressions. Figure out why.
        // I think it may have to do with boxed fat pointers.
        // https://github.com/model-checking/kani/issues/277
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

impl<'tcx> TypeOrVariant<'tcx> {
    pub fn monomorphize(self, ctx: &GotocCtx<'tcx>) -> Self {
        match self {
            TypeOrVariant::Type(t) => TypeOrVariant::Type(ctx.monomorphize(t)),
            TypeOrVariant::Variant(_) | TypeOrVariant::GeneratorVariant(_) => self,
        }
    }
}

impl<'tcx> TypeOrVariant<'tcx> {
    pub fn expect_type(&self) -> Ty<'tcx> {
        match self {
            TypeOrVariant::Type(t) => *t,
            TypeOrVariant::Variant(v) => panic!("expect a type but variant is found: {v:?}"),
            TypeOrVariant::GeneratorVariant(v) => {
                panic!("expect a type but generator variant is found: {v:?}")
            }
        }
    }

    #[allow(dead_code)]
    pub fn expect_variant(&self) -> &'tcx VariantDef {
        match self {
            TypeOrVariant::Type(t) => panic!("expect a variant but type is found: {t:?}"),
            TypeOrVariant::Variant(v) => v,
            TypeOrVariant::GeneratorVariant(v) => {
                panic!("expect a variant but generator variant found {v:?}")
            }
        }
    }
}

impl<'tcx> GotocCtx<'tcx> {
    fn codegen_field(
        &mut self,
        parent_expr: Expr,
        parent_ty_or_var: TypeOrVariant<'tcx>,
        field: &FieldIdx,
        field_ty_or_var: TypeOrVariant<'tcx>,
    ) -> Result<Expr, UnimplementedData> {
        match parent_ty_or_var {
            TypeOrVariant::Type(parent_ty) => {
                match parent_ty.kind() {
                    ty::Alias(..)
                    | ty::Bool
                    | ty::Char
                    | ty::Int(_)
                    | ty::Uint(_)
                    | ty::Float(_)
                    | ty::FnPtr(_)
                    | ty::Never
                    | ty::FnDef(..)
                    | ty::GeneratorWitness(..)
                    | ty::Foreign(..)
                    | ty::Dynamic(..)
                    | ty::Bound(..)
                    | ty::Placeholder(..)
                    | ty::Param(_)
                    | ty::Infer(_)
                    | ty::Error(_) => unreachable!("type {:?} does not have a field", parent_ty),
                    ty::Tuple(_) => Ok(parent_expr
                        .member(&Self::tuple_fld_name(field.index()), &self.symbol_table)),
                    ty::Adt(def, _) if def.repr().simd() => Ok(self.codegen_simd_field(
                        parent_expr,
                        *field,
                        field_ty_or_var.expect_type(),
                    )),
                    // if we fall here, then we are handling either a struct or a union
                    ty::Adt(def, _) => {
                        let field = &def.variants().raw[0].fields[*field];
                        Ok(parent_expr.member(&field.name.to_string(), &self.symbol_table))
                    }
                    ty::Closure(..) => {
                        Ok(parent_expr.member(&field.index().to_string(), &self.symbol_table))
                    }
                    ty::Generator(..) => {
                        let field_name = self.generator_field_name(field.as_usize());
                        Ok(parent_expr
                            .member("direct_fields", &self.symbol_table)
                            .member(field_name, &self.symbol_table))
                    }
                    _ => unimplemented!(),
                }
            }
            // if we fall here, then we are handling an enum
            TypeOrVariant::Variant(parent_var) => {
                let field = &parent_var.fields[*field];
                Ok(parent_expr.member(&field.name.to_string(), &self.symbol_table))
            }
            TypeOrVariant::GeneratorVariant(_var_idx) => {
                let field_name = self.generator_field_name(field.index());
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
    fn codegen_simd_field(
        &mut self,
        parent_expr: Expr,
        field: FieldIdx,
        field_ty: Ty<'tcx>,
    ) -> Expr {
        if matches!(field_ty.kind(), ty::Array { .. }) {
            // Array based
            assert_eq!(field.index(), 0);
            let field_typ = self.codegen_ty(field_ty);
            parent_expr.reinterpret_cast(field_typ)
        } else {
            // Return the given field.
            let index_expr = Expr::int_constant(field.index(), Type::size_t());
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
    fn codegen_local_fndef(&mut self, ty: ty::Ty<'tcx>) -> Option<Expr> {
        match ty.kind() {
            // A local that is itself a FnDef, like Fn::call_once
            ty::FnDef(defid, substs) => Some(self.codegen_fndef(*defid, substs, None)),
            // A local can be pointer to a FnDef, like Fn::call and Fn::call_mut
            ty::RawPtr(inner) => self
                .codegen_local_fndef(inner.ty)
                .map(|f| if f.can_take_address_of() { f.address_of() } else { f }),
            // A local can be a boxed function pointer
            ty::Adt(def, _) if def.is_box() => {
                let boxed_ty = self.codegen_ty(ty);
                self.codegen_local_fndef(ty.boxed_ty())
                    .map(|f| self.box_value(f.address_of(), boxed_ty))
            }
            _ => None,
        }
    }

    /// Codegen for a local
    fn codegen_local(&mut self, l: Local) -> Expr {
        // Check if the local is a function definition (see comment above)
        if let Some(fn_def) = self.codegen_local_fndef(self.local_ty(l)) {
            return fn_def;
        }

        // Otherwise, simply look up the local by the var name.
        let vname = self.codegen_var_name(&l);
        Expr::symbol_expression(vname, self.codegen_ty(self.local_ty(l)))
    }

    /// A projection is an operation that translates an lvalue to another lvalue.
    /// E.g. dereference, follow a field, etc.
    /// This function codegens a single step of a projection.
    /// `before` is the expression "before" this projection is applied;
    /// the return value is the expression after.
    fn codegen_projection(
        &mut self,
        before: Result<ProjectedPlace<'tcx>, UnimplementedData>,
        proj: ProjectionElem<Local, Ty<'tcx>>,
    ) -> Result<ProjectedPlace<'tcx>, UnimplementedData> {
        let before = before?;
        match proj {
            ProjectionElem::Deref => {
                trace!(?before, ?proj, "codegen_projection");
                let base_type = before.mir_typ();
                let inner_goto_expr = if base_type.is_box() {
                    self.deref_box(before.goto_expr)
                } else {
                    before.goto_expr
                };

                let inner_mir_typ = std_pointee_type(base_type).unwrap();
                let (fat_ptr_mir_typ, fat_ptr_goto_expr) = if self.use_thin_pointer(inner_mir_typ) {
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
                        self.use_fat_pointer(pointee_type(fat_ptr_mir_typ.unwrap()).unwrap()),
                        "Unexpected type: {:?} -- {:?}",
                        fat_ptr.typ(),
                        fat_ptr_mir_typ,
                    );
                };

                let expr = match inner_mir_typ.kind() {
                    ty::Slice(_) | ty::Str | ty::Dynamic(..) => {
                        inner_goto_expr.member("data", &self.symbol_table)
                    }
                    ty::Adt(..) if self.is_unsized(inner_mir_typ) => {
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
                            .cast_to(self.codegen_ty(inner_mir_typ).to_pointer())
                            .dereference()
                    }
                    _ => inner_goto_expr.dereference(),
                };
                let typ = TypeOrVariant::Type(inner_mir_typ);
                ProjectedPlace::try_new(expr, typ, fat_ptr_goto_expr, fat_ptr_mir_typ, self)
            }
            ProjectionElem::Field(f, t) => {
                let typ = TypeOrVariant::Type(t);
                let expr =
                    self.codegen_field(before.goto_expr, before.mir_typ_or_variant, &f, typ)?;
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
                let idxe = self.codegen_local(i);
                let typ = match base_type.kind() {
                    ty::Array(elemt, _) | ty::Slice(elemt) => TypeOrVariant::Type(*elemt),
                    _ => unreachable!("must index an array"),
                };
                let expr = match base_type.kind() {
                    ty::Array(..) => self.codegen_idx_array(before.goto_expr, idxe),
                    ty::Slice(..) => before.goto_expr.index(idxe),
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
                self.codegen_constant_index(before, offset, min_length, from_end)
            }
            // Best effort to codegen subslice projection.
            // Full support to be added in
            // https://github.com/model-checking/kani/issues/707
            ProjectionElem::Subslice { from, to, from_end } => {
                // https://rust-lang.github.io/rfcs/2359-subslice-pattern-syntax.html
                match before.mir_typ().kind() {
                    ty::Array(ty, len) => {
                        let len = len.kind().try_to_target_usize(self.tcx).unwrap();
                        let subarray_len = if from_end {
                            // `to` counts from the end of the array
                            len - to - from
                        } else {
                            to - from
                        };
                        let typ = self.tcx.mk_array(*ty, subarray_len);
                        let goto_typ = self.codegen_ty(typ);
                        // unimplemented
                        Err(UnimplementedData::new(
                            "Sub-array binding",
                            "https://github.com/model-checking/kani/issues/707",
                            goto_typ,
                            *before.goto_expr.location(),
                        ))
                    }
                    ty::Slice(elemt) => {
                        let len = if from_end {
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
                        let typ = self.tcx.mk_slice(*elemt);
                        let typ_and_mut = TypeAndMut { ty: typ, mutbl: Mutability::Mut };
                        let ptr_typ = self.tcx.mk_ptr(typ_and_mut);
                        let goto_type = self.codegen_ty(ptr_typ);

                        let index = Expr::int_constant(from, Type::ssize_t());
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
            ProjectionElem::Downcast(_, idx) => {
                // downcast converts a variable of an enum type to one of its discriminated cases
                let t = before.mir_typ();
                let (case_name, type_or_variant) = match t.kind() {
                    ty::Adt(def, _) => {
                        let variant = def.variant(idx);
                        (variant.name.as_str().into(), TypeOrVariant::Variant(variant))
                    }
                    ty::Generator(..) => {
                        (self.generator_variant_name(idx), TypeOrVariant::GeneratorVariant(idx))
                    }
                    _ => unreachable!(
                        "cannot downcast {:?} to a variant (only enums and generators can)",
                        &t.kind()
                    ),
                };
                let layout = self.layout_of(t);
                let expr = match &layout.variants {
                    Variants::Single { .. } => before.goto_expr,
                    Variants::Multiple { tag_encoding, .. } => match tag_encoding {
                        TagEncoding::Direct => {
                            let cases = if t.is_generator() {
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
            ProjectionElem::OpaqueCast(ty) => ProjectedPlace::try_new(
                before.goto_expr.cast_to(self.codegen_ty(ty)),
                TypeOrVariant::Type(ty),
                before.fat_ptr_goto_expr,
                before.fat_ptr_mir_typ,
                self,
            ),
        }
    }

    /// Codegen the reference to a given place.
    /// We currently have a somewhat weird way of handling ZST.
    /// - For `*(&T)` where `T: Unsized`, the projection's `goto_expr` is a thin pointer, so we
    ///   build the fat pointer from there.
    /// - For `*(Wrapper<T>)` where `T: Unsized`, the projection's `goto_expr` returns an object,
    ///   and we need to take it's address and build the fat pointer.
    pub fn codegen_place_ref(&mut self, place: &Place<'tcx>) -> Expr {
        let place_ty = self.place_ty(place);
        let projection = unwrap_or_return_codegen_unimplemented!(self, self.codegen_place(place));
        if self.use_thin_pointer(place_ty) {
            // Just return the address of the place dereferenced.
            projection.goto_expr.address_of()
        } else if place_ty == pointee_type(self.local_ty(place.local)).unwrap() {
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
    pub fn codegen_place(
        &mut self,
        p: &Place<'tcx>,
    ) -> Result<ProjectedPlace<'tcx>, UnimplementedData> {
        debug!(place=?p, "codegen_place");
        let initial_expr = self.codegen_local(p.local);
        let initial_typ = TypeOrVariant::Type(self.local_ty(p.local));
        debug!(?initial_typ, ?initial_expr, "codegen_place");
        let initial_projection =
            ProjectedPlace::try_new(initial_expr, initial_typ, None, None, self);
        let result = p
            .projection
            .iter()
            .fold(initial_projection, |accum, proj| self.codegen_projection(accum, proj));
        match result {
            Err(data) => Err(UnimplementedData::new(
                &data.operation,
                &data.bug_url,
                self.codegen_ty(self.place_ty(p)),
                data.loc,
            )),
            _ => result,
        }
    }

    /// Given a projection, generate an lvalue that represents the given variant index.
    pub fn codegen_variant_lvalue(
        &mut self,
        initial_projection: ProjectedPlace<'tcx>,
        variant_idx: VariantIdx,
    ) -> ProjectedPlace<'tcx> {
        debug!(?initial_projection, ?variant_idx, "codegen_variant_lvalue");
        let downcast = ProjectionElem::Downcast(None, variant_idx);
        self.codegen_projection(Ok(initial_projection), downcast).unwrap()
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
        before: ProjectedPlace<'tcx>,
        offset: u64,
        min_length: u64,
        from_end: bool,
    ) -> Result<ProjectedPlace<'tcx>, UnimplementedData> {
        match before.mir_typ().kind() {
            //TODO, ask on zulip if we can ever have from_end here?
            ty::Array(elemt, length) => {
                let length = length.kind().try_to_target_usize(self.tcx).unwrap();
                assert!(length >= min_length);
                let idx = if from_end { length - offset } else { offset };
                let idxe = Expr::int_constant(idx, Type::ssize_t());
                let expr = self.codegen_idx_array(before.goto_expr, idxe);
                let typ = TypeOrVariant::Type(*elemt);
                ProjectedPlace::try_new(
                    expr,
                    typ,
                    before.fat_ptr_goto_expr,
                    before.fat_ptr_mir_typ,
                    self,
                )
            }
            ty::Slice(elemt) => {
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
                let typ = TypeOrVariant::Type(*elemt);
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
