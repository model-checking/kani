// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! responsible for handling codegening places.
//!
//! a place is an expression of specifying a location in memory, like a left value. check the cases
//! in [codegen_place] below.

use super::cbmc::goto_program::{Expr, Type};
use super::metadata::*;
use super::typ::tuple_fld;
use rustc_middle::{
    mir::{Field, Local, Place, ProjectionElem},
    ty::{self, Ty, TyS, VariantDef},
};
use rustc_target::abi::{LayoutOf, TagEncoding, Variants};
use tracing::{debug, warn};

/// A projection in RMC can either be to a type (the normal case),
/// or a variant in the case of a downcast.
#[derive(Debug)]
pub enum TypeOrVariant<'tcx> {
    Type(Ty<'tcx>),
    Variant(&'tcx VariantDef),
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
    fn check_expr_typ(expr: &Expr, typ: &TypeOrVariant<'tcx>, ctx: &mut GotocCtx<'tcx>) -> bool {
        match typ {
            TypeOrVariant::Type(t) => &ctx.codegen_ty(t) == expr.typ(),
            TypeOrVariant::Variant(_) => true, //TODO, what to do here?
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

    pub fn new(
        goto_expr: Expr,
        mir_typ_or_variant: TypeOrVariant<'tcx>,
        fat_ptr_goto_expr: Option<Expr>,
        fat_ptr_mir_typ: Option<Ty<'tcx>>,
        ctx: &mut GotocCtx<'tcx>,
    ) -> Self {
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
        // https://github.com/model-checking/rmc/issues/277
        if !Self::check_expr_typ(&goto_expr, &mir_typ_or_variant, ctx) {
            warn!(
                "Unexpected type mismatch in projection: \n{:?}\n{:?}",
                &goto_expr, &mir_typ_or_variant
            );
        };

        assert!(
            Self::check_fat_ptr_typ(&fat_ptr_goto_expr, &fat_ptr_mir_typ, ctx),
            "\n{:?}\n{:?}",
            &fat_ptr_goto_expr,
            &fat_ptr_mir_typ
        );
        ProjectedPlace { goto_expr, mir_typ_or_variant, fat_ptr_goto_expr, fat_ptr_mir_typ }
    }
}

impl<'tcx> TypeOrVariant<'tcx> {
    pub fn monomorphize(self, ctx: &GotocCtx<'tcx>) -> Self {
        match self {
            TypeOrVariant::Type(t) => TypeOrVariant::Type(ctx.monomorphize(t)),
            TypeOrVariant::Variant(_) => self,
        }
    }
}

impl<'tcx> TypeOrVariant<'tcx> {
    pub fn expect_type(&self) -> Ty<'tcx> {
        match self {
            TypeOrVariant::Type(t) => t,
            TypeOrVariant::Variant(v) => panic!("expect a type but variant is found: {:?}", v),
        }
    }

    pub fn expect_variant(&self) -> &'tcx VariantDef {
        match self {
            TypeOrVariant::Type(t) => panic!("expect a variant but type is found: {:?}", t),
            TypeOrVariant::Variant(v) => v,
        }
    }
}

impl<'tcx> GotocCtx<'tcx> {
    fn codegen_field(&mut self, res: Expr, t: TypeOrVariant<'tcx>, f: &Field) -> Expr {
        match t {
            TypeOrVariant::Type(t) => {
                match t.kind() {
                    ty::Bool
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
                    | ty::Projection(_)
                    | ty::Bound(..)
                    | ty::Placeholder(..)
                    | ty::Opaque(..)
                    | ty::Param(_)
                    | ty::Infer(_)
                    | ty::Error(_) => unreachable!("type {:?} does not have a field", t),
                    ty::Tuple(_) => res.member(&tuple_fld(f.index()), &self.symbol_table),
                    ty::Adt(def, _) if def.repr.simd() => {
                        // this is a SIMD vector - the index represents one
                        // of the elements, so we want to index as an array
                        // Example:
                        // pub struct i64x2(i64, i64);
                        // fn main() {
                        //   let v = i64x2(1, 2);
                        //   assert!(v.0 == 1); // refers to the first i64
                        //   assert!(v.1 == 2);
                        // }
                        let size_index = Expr::int_constant(f.index(), Type::size_t());
                        res.index_array(size_index)
                    }
                    // if we fall here, then we are handling either a struct or a union
                    ty::Adt(def, _) => {
                        let field = &def.variants.raw[0].fields[f.index()];
                        res.member(&field.ident.name.to_string(), &self.symbol_table)
                    }
                    ty::Closure(..) => res.member(&f.index().to_string(), &self.symbol_table),
                    _ => unimplemented!(),
                }
            }
            // if we fall here, then we are handling an enum
            TypeOrVariant::Variant(v) => {
                let field = &v.fields[f.index()];
                res.member(&field.ident.name.to_string(), &self.symbol_table)
            }
        }
    }

    /// If a local is a function definition, ignore the local variable name and
    /// generate a function call based on the def id.
    ///
    /// Note that this is finicky. A local might be a function definition or a
    /// pointer to one. For example, the auto-generated code for Fn::call_once
    /// uses a local FnDef to call the wrapped function, while the auto-generated
    /// code for Fn::call and Fn::call_mut both use pointers to a FnDef.
    /// In these cases, we need to generate an expression that references the
    /// existing fndef rather than a named variable.
    pub fn codegen_local_fndef(&mut self, l: Local) -> Option<Expr> {
        let t = self.local_ty(l);
        match t.kind() {
            // A local that is itself a FnDef, like Fn::call_once
            ty::FnDef(defid, substs) => Some(self.codegen_fndef(*defid, substs, None)),
            // A local that is a pointer to a FnDef, like Fn::call and Fn::call_mut
            ty::RawPtr(inner) => match inner.ty.kind() {
                ty::FnDef(defid, substs) => {
                    Some(self.codegen_fndef(*defid, substs, None).address_of())
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Codegen for a local
    pub fn codegen_local(&mut self, l: Local) -> Expr {
        // Check if the local is a function definition (see comment above)
        if let Some(fn_def) = self.codegen_local_fndef(l) {
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
        before: ProjectedPlace<'tcx>,
        proj: ProjectionElem<Local, &'tcx TyS<'_>>,
    ) -> ProjectedPlace<'tcx> {
        match proj {
            ProjectionElem::Deref => {
                let base_type = before.mir_typ();
                let inner_goto_expr = if base_type.is_box() {
                    self.deref_box(before.goto_expr)
                } else {
                    before.goto_expr
                };

                let inner_mir_typ_and_mut = base_type.builtin_deref(true).unwrap();
                let fat_ptr_mir_typ = if self.is_box_of_unsized(base_type) {
                    assert!(before.fat_ptr_mir_typ.is_none());
                    // If we have a box, its fat pointer typ is a pointer to the boxes inner type.
                    Some(self.tcx.mk_ptr(inner_mir_typ_and_mut))
                } else if self.is_ref_of_unsized(base_type) {
                    assert!(before.fat_ptr_mir_typ.is_none());
                    Some(before.mir_typ_or_variant.expect_type())
                } else {
                    before.fat_ptr_mir_typ
                };

                let fat_ptr_goto_expr =
                    if self.is_box_of_unsized(base_type) || self.is_ref_of_unsized(base_type) {
                        assert!(before.fat_ptr_goto_expr.is_none());
                        Some(inner_goto_expr.clone())
                    } else {
                        before.fat_ptr_goto_expr
                    };

                // Check that we have a valid trait or slice fat pointer
                if let Some(fat_ptr) = fat_ptr_goto_expr.clone() {
                    assert!(
                        fat_ptr.typ().is_rust_trait_fat_ptr(&self.symbol_table)
                            || fat_ptr.typ().is_rust_slice_fat_ptr(&self.symbol_table)
                    );
                };

                let inner_mir_typ = inner_mir_typ_and_mut.ty;
                let expr = match inner_mir_typ.kind() {
                    ty::Slice(_) | ty::Str | ty::Dynamic(..) => {
                        inner_goto_expr.member("data", &self.symbol_table)
                    }
                    ty::Adt(..) if self.is_unsized(inner_mir_typ) => {
                        // in cbmc-reg/Strings/os_str_reduced.rs, we see
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
                ProjectedPlace::new(expr, typ, fat_ptr_goto_expr, fat_ptr_mir_typ, self)
            }
            ProjectionElem::Field(f, t) => {
                let typ = TypeOrVariant::Type(t);
                let expr = self.codegen_field(before.goto_expr, before.mir_typ_or_variant, &f);
                ProjectedPlace::new(
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
                    ty::Array(elemt, _) | ty::Slice(elemt) => TypeOrVariant::Type(elemt),
                    _ => unreachable!("must index an array"),
                };
                let expr = match base_type.kind() {
                    ty::Array(..) => self.codegen_idx_array(before.goto_expr, idxe),
                    ty::Slice(..) => before.goto_expr.index(idxe),
                    _ => unreachable!("must index an array"),
                };
                ProjectedPlace::new(
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
            ProjectionElem::Subslice { .. } => unimplemented!(),
            ProjectionElem::Downcast(_, idx) => {
                // downcast converts a variable of an enum type to one of its discriminated cases
                let t = before.mir_typ();
                match t.kind() {
                    ty::Adt(def, _) => {
                        let variant = def.variants.get(idx).unwrap();
                        let typ = TypeOrVariant::Variant(variant);
                        let expr = match &self.layout_of(t).variants {
                            Variants::Single { .. } => before.goto_expr,
                            Variants::Multiple { tag_encoding, .. } => match tag_encoding {
                                TagEncoding::Direct => {
                                    let case_name = variant.ident.name.to_string();
                                    before
                                        .goto_expr
                                        .member("cases", &self.symbol_table)
                                        .member(&case_name, &self.symbol_table)
                                }
                                TagEncoding::Niche { .. } => before.goto_expr,
                            },
                        };
                        ProjectedPlace::new(
                            expr,
                            typ,
                            before.fat_ptr_goto_expr,
                            before.fat_ptr_mir_typ,
                            self,
                        )
                    }
                    _ => unreachable!("it's a bug to reach here!"),
                }
            }
        }
    }

    /// Given a MIR place, generate a CBMC expression that represents it as a CBMC lvalue.
    /// A place is the rust term for an lvalue.
    /// Like in "C", a place can be a "projected": e.g. `*x.foo = bar`
    /// This function follows the MIR projection to get the final useable lvalue.
    /// If it passes through a fat pointer along the way, it stores info about it,
    /// which can be useful in reconstructing fat pointer operations.
    pub fn codegen_place(&mut self, p: &Place<'tcx>) -> ProjectedPlace<'tcx> {
        debug!("codegen_place: {:?}", p);
        let initial_expr = self.codegen_local(p.local);
        let initial_typ = TypeOrVariant::Type(self.local_ty(p.local));
        let initial_projection = ProjectedPlace::new(initial_expr, initial_typ, None, None, self);
        p.projection
            .iter()
            .fold(initial_projection, |accum, proj| self.codegen_projection(accum, proj))
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
    ) -> ProjectedPlace<'tcx> {
        match before.mir_typ().kind() {
            //TODO, ask on zulip if we can ever have from_end here?
            ty::Array(elemt, length) => {
                let length = length.val.try_to_machine_usize(self.tcx).unwrap();
                assert!(length >= min_length);
                let idx = if from_end { length - offset } else { offset };
                let idxe = Expr::int_constant(idx, Type::ssize_t());
                let expr = self.codegen_idx_array(before.goto_expr, idxe);
                let typ = TypeOrVariant::Type(elemt);
                ProjectedPlace::new(
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
                let typ = TypeOrVariant::Type(elemt);
                ProjectedPlace::new(
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
        arr.member("0", &self.symbol_table).index_array(idx)
    }
}
