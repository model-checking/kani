// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module defines functions which impose data invariant on generated data types.

use super::cbmc::goto_program::{Expr, Location, Stmt, Symbol, Type};
use super::metadata::GotocCtx;
use crate::gotoc::typ::tuple_fld;
use rustc_middle::mir::interpret::{ConstValue, Scalar};
use rustc_middle::ty;
use rustc_middle::ty::ScalarInt;
use rustc_middle::ty::Ty;
use rustc_middle::ty::{IntTy, UintTy};
use rustc_target::abi::{FieldsShape, LayoutOf, Primitive, TagEncoding, Variants};

fn fold_invariants_gen<F: Fn(Expr, Expr) -> Expr>(mut iv: Vec<Expr>, dfl: Expr, comb: F) -> Expr {
    let mut res: Option<Expr> = None;
    while !iv.is_empty() {
        let e = iv.remove(0);
        if let Some(r) = res {
            res = Some(comb(r, e));
        } else {
            res = Some(e);
        }
    }
    res.unwrap_or(dfl)
}

fn fold_invariants(iv: Vec<Expr>) -> Expr {
    fold_invariants_gen(iv, Expr::bool_true(), |a, b| a.and(b))
}

fn fold_invariants_or(iv: Vec<Expr>) -> Expr {
    fold_invariants_gen(iv, Expr::bool_false(), |a, b| a.or(b))
}

impl<'tcx> GotocCtx<'tcx> {
    fn invariant_name(&mut self, t: Ty<'tcx>) -> String {
        let ty_name = self.ty_mangled_name(t);
        let fname = format!("{}:invariant", ty_name);
        fname
    }

    fn bound_ty_above_and_below(
        &mut self,
        fname: String,
        t: Ty<'tcx>,
        below: i32,
        above: i32,
    ) -> Option<Expr> {
        self.ensure(&fname, |ctx, _| {
            ctx.codegen_assumption_genfunc(&fname, t, |_tcx, ptr, body| {
                let exp = ptr
                    .clone()
                    .dereference()
                    .ge(Expr::int_constant(below, Type::signed_int(32)))
                    .and(ptr.dereference().le(Expr::int_constant(above, Type::signed_int(32))))
                    .ret(Location::none());
                body.push(exp)
            })
        });
        self.find_function(&fname)
    }

    fn bound_true_false(&mut self, fname: String, t: Ty<'tcx>) -> Option<Expr> {
        self.ensure(&fname, |ctx, _| {
            ctx.codegen_assumption_genfunc(&fname, t, |_tcx, ptr, body| {
                let exp = ptr
                    .clone()
                    .dereference()
                    .eq(Expr::c_true())
                    .or(ptr.dereference().eq(Expr::c_false()))
                    .ret(Location::none());
                body.push(exp)
            })
        });
        self.find_function(&fname)
    }
    /// return an option of an expression representing a function assuming the data invariant of the type t
    ///
    /// for a type t, this function generates
    ///     bool t:invariant(t *x);
    pub fn codegen_assumption(&mut self, t: Ty<'tcx>) -> Option<Expr> {
        let fname = self.invariant_name(t);
        match t.kind() {
            ty::Bool => self.bound_true_false(fname, t),
            ty::Int(_) | ty::Uint(_) | ty::Float(_) => None,
            ty::Char => {
                //Char is an i32, but has invalid values. This means that rust can do a niche optimization for
                // Option<char>. Make sure we never generate a value above char::MAX  https://doc.rust-lang.org/beta/std/char/constant.MAX.html
                self.bound_ty_above_and_below(fname, t, 0, '\u{10ffff}' as i32)
            }
            ty::Adt(def, subst) => {
                if def.is_union() {
                    None
                }
                //
                else if def.is_struct() {
                    let variant = &def.variants.raw[0];
                    self.ensure(&fname, |ctx, _| {
                        ctx.codegen_assumption_struct(&fname, t, variant, subst)
                    });
                    self.find_function(&fname)
                } else {
                    // is enum
                    if def.variants.is_empty() {
                        None
                    } else {
                        self.ensure(&fname, |ctx, _| {
                            ctx.codegen_assumption_enum(&fname, t, def, subst)
                        });
                        self.find_function(&fname)
                    }
                }
            }
            ty::Foreign(_) => unreachable!("cannot generate assumptions for foreign types"),
            ty::Array(et, c) => {
                self.ensure(&fname, |ctx, _| ctx.codegen_assumption_array(&fname, t, et, c));
                self.find_function(&fname)
            }
            ty::Str | ty::Slice(_) => unreachable!("should be handled by Ref or RawPtr"),
            ty::RawPtr(rt) => self.codegen_assumption_ref_ptr(&fname, t, rt.ty, false),
            ty::Ref(_, rt, _) => self.codegen_assumption_ref_ptr(&fname, t, rt, true),
            ty::FnDef(_, _) | ty::FnPtr(_) => None,
            ty::Dynamic(_, _) => unreachable!(),
            ty::Closure(_, _) => None,
            ty::Generator(_, _, _) => unimplemented!(),
            ty::GeneratorWitness(_) => unimplemented!(),
            ty::Never => None,
            ty::Tuple(ts) => {
                if ts.is_empty() {
                    None
                } else {
                    self.ensure(&fname, |ctx, _| ctx.codegen_assumption_tuple(&fname, t, ts));
                    self.find_function(&fname)
                }
            }
            ty::Projection(_) | ty::Opaque(_, _) => {
                let normalized = self.tcx.normalize_erasing_regions(ty::ParamEnv::reveal_all(), t);
                self.codegen_assumption(normalized)
            }
            ty::Bound(_, _) | ty::Param(_) => unreachable!("monomorphization bug"),
            ty::Placeholder(_) | ty::Infer(_) | ty::Error(_) => {
                unreachable!("remnants of type checking")
            }
        }
    }

    /// * fname - function name
    /// * t - type of a reference
    /// * rt - type of the referenced term
    /// * is_ref - whether we are handling references or pointers
    fn codegen_assumption_ref_ptr(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        rt: Ty<'tcx>,
        is_ref: bool,
    ) -> Option<Expr> {
        match rt.kind() {
            ty::Slice(e) => {
                let ef = self.codegen_assumption(e);
                if ef.is_none() && !is_ref {
                    None
                } else {
                    self.ensure(&fname, |ctx, _| {
                        ctx.codegen_assumption_ref_ptr_slice(fname, t, ef, is_ref)
                    });
                    self.find_function(fname)
                }
            }
            ty::Str => {
                if !is_ref {
                    None
                } else {
                    self.ensure(&fname, |ctx, _| {
                        ctx.codegen_assumption_ref_ptr_slice(fname, t, None, is_ref)
                    });
                    self.find_function(fname)
                }
            }
            ty::Dynamic(_, _) => unimplemented!(),
            ty::Projection(_) | ty::Opaque(_, _) => {
                let normalized = self.tcx.normalize_erasing_regions(ty::ParamEnv::reveal_all(), t);
                self.codegen_assumption_ref_ptr(fname, t, normalized, is_ref)
            }
            _ => {
                let ef = self.codegen_assumption(rt);
                if ef.is_none() && !is_ref {
                    None
                } else {
                    self.ensure(&fname, |ctx, _| {
                        ctx.codegen_assumption_ref_ptr_thin(fname, t, ef, is_ref)
                    });
                    self.find_function(fname)
                }
            }
        }
    }

    /// * fname - function name
    /// * t - type of a reference to a slice
    /// * e - type of an element in the slice
    /// * f - invariant function for each element
    /// * is_ref - whether we are handling references or pointers
    fn codegen_assumption_ref_ptr_slice(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        f: Option<Expr>,
        is_ref: bool,
    ) -> Symbol {
        self.codegen_assumption_genfunc(fname, t, |tcx, ptr, body| {
            let sl = ptr.dereference();
            let data = sl.clone().member("data", &tcx.symbol_table);
            let len = sl.member("len", &tcx.symbol_table);
            let mut invariants = vec![];
            if is_ref {
                invariants.push(data.clone().is_nonnull());
            }
            if let Some(f) = f {
                //CHECKME: why is this 2?
                let idx = tcx.gen_function_local_variable(2, &fname, Type::size_t()).to_expr();
                body.push(Stmt::decl(idx.clone(), Some(Type::size_t().zero()), Location::none()));
                let lbody = Stmt::block(
                    vec![
                        data.clone()
                            .is_nonnull()
                            .implies(f.call(vec![data.plus(idx.clone())]))
                            .not()
                            .if_then_else(
                                Expr::bool_false().ret(Location::none()),
                                None,
                                Location::none(),
                            ),
                    ],
                    Location::none(),
                );
                body.push(Stmt::for_loop(
                    Stmt::skip(Location::none()),
                    idx.clone().lt(len),
                    idx.postincr().as_stmt(Location::none()),
                    lbody,
                    Location::none(),
                ));
            }
            body.push(fold_invariants(invariants).ret(Location::none()));
        })
    }

    /// * fname - function name
    /// * t - type of the reference
    /// * f - invariant function for the referenced element
    /// * is_ref - whether we are handling references or pointers
    fn codegen_assumption_ref_ptr_thin(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        f: Option<Expr>,
        is_ref: bool,
    ) -> Symbol {
        self.codegen_assumption_genfunc(fname, t, |_, ptr, body| {
            let x = ptr.dereference();
            let mut invarints = vec![];
            if is_ref {
                invarints.push(x.clone().is_nonnull());
            }
            if let Some(f) = f {
                invarints.push(x.clone().is_nonnull().implies(f.call(vec![x])));
            }
            body.push(fold_invariants(invarints).ret(Location::none()));
        })
    }

    fn codegen_assumption_struct_invariant(
        &mut self,
        ptr: Expr,
        variant: &ty::VariantDef,
        subst: ty::subst::SubstsRef<'tcx>,
    ) -> Vec<Expr> {
        let mut invariants = vec![];
        for fd in &variant.fields {
            let t = fd.ty(self.tcx, subst);
            if let Some(f) = self.codegen_assumption(t) {
                let fp = ptr
                    .clone()
                    .dereference()
                    .member(&fd.ident.name.to_string(), &self.symbol_table)
                    .address_of();
                let assumption = f.call(vec![fp]);
                invariants.push(assumption);
            }
        }
        invariants
    }

    /// collect invariant for each field
    fn codegen_assumption_struct(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        variant: &'tcx ty::VariantDef,
        subst: ty::subst::SubstsRef<'tcx>,
    ) -> Symbol {
        self.codegen_assumption_genfunc(fname, t, |tcx, ptr, body| {
            let invariants = tcx.codegen_assumption_struct_invariant(ptr, variant, subst);
            body.push(fold_invariants(invariants).ret(Location::none()));
        })
    }

    fn codegen_assumption_enum(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        def: &'tcx ty::AdtDef,
        subst: ty::subst::SubstsRef<'tcx>,
    ) -> Symbol {
        match def.variants.len() {
            0 => unreachable!(),
            1 => self.codegen_assumption_enum_single_variant(fname, t, &def.variants.raw[0], subst),
            _ => {
                let layout = self.layout_of(t);
                match &layout.variants {
                    Variants::Single { .. } => unreachable!(),
                    Variants::Multiple { tag_encoding, .. } => match tag_encoding {
                        TagEncoding::Direct => {
                            self.codegen_assumption_enum_direct(fname, t, def, subst)
                        }
                        TagEncoding::Niche { .. } => {
                            self.codegen_assumption_enum_niche(fname, t, def, subst)
                        }
                    },
                }
            }
        }
    }

    fn codegen_assumption_enum_single_variant(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        variant: &ty::VariantDef,
        subst: ty::subst::SubstsRef<'tcx>,
    ) -> Symbol {
        self.codegen_assumption_genfunc(fname, t, |tcx, ptr, body| {
            let invariants = tcx.codegen_assumption_struct_invariant(ptr, variant, subst);
            body.push(fold_invariants(invariants).ret(Location::none()));
        })
    }

    fn codegen_assumption_enum_niche(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        def: &'tcx ty::AdtDef,
        subst: ty::subst::SubstsRef<'tcx>,
    ) -> Symbol {
        let layout = self.layout_of(t);
        let (tag, dataful_variant, niche_variants, niche_start) = match &layout.variants {
            Variants::Single { .. } => unreachable!(),
            Variants::Multiple { tag, tag_encoding, .. } => match tag_encoding {
                TagEncoding::Direct => unreachable!(),
                TagEncoding::Niche { dataful_variant, niche_variants, niche_start } => {
                    (tag, dataful_variant, niche_variants, niche_start)
                }
            },
        };
        let offset = match &layout.fields {
            FieldsShape::Arbitrary { offsets, .. } => offsets[0].bytes_usize(),
            _ => unreachable!("niche encoding must have arbitrary fields"),
        };
        let discr_ty = self.codegen_enum_discr_typ(t);
        let discr_ty = self.codegen_ty(discr_ty);
        let variant = &def.variants[*dataful_variant];
        self.codegen_assumption_genfunc(fname, t, |tcx, ptr, body| {
            let discr = tcx.codegen_get_niche(ptr.clone().dereference(), offset, discr_ty.clone());
            let mut invariants = vec![];
            for (idx, _) in def.variants.iter_enumerated() {
                if idx != *dataful_variant {
                    let niche_value = idx.as_u32() - niche_variants.start().as_u32();
                    let niche_value = (niche_value as u128).wrapping_add(*niche_start);
                    let value = if niche_value == 0 && tag.value == Primitive::Pointer {
                        discr_ty.null()
                    } else {
                        Expr::int_constant(niche_value, discr_ty.clone())
                    };
                    invariants.push(discr.clone().eq(value));
                }
            }

            let data_invar = tcx.codegen_assumption_struct_invariant(ptr, variant, subst);
            invariants.push(fold_invariants(data_invar));
            body.push(fold_invariants_or(invariants).ret(Location::none()));
        })
    }

    /// see comments in the body
    fn codegen_assumption_enum_direct(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        def: &'tcx ty::AdtDef,
        subst: ty::subst::SubstsRef<'tcx>,
    ) -> Symbol {
        self.codegen_assumption_genfunc(fname, t, |tcx, ptr, body| {
            // here we have enum
            //
            // we have a few invariants to assume.
            // 1. the discriminant is within [0, def.variants.len())
            let discr_t = tcx.codegen_enum_discr_typ(t);
            let isz = tcx.codegen_ty(discr_t);
            let case = ptr.clone().dereference().member("case", &tcx.symbol_table);
            let mut invariants = vec![];
            if def
                .variants
                .iter_enumerated()
                .all(|(i, v)| v.discr == ty::VariantDiscr::Relative(i.as_u32()))
            {
                // ptr.case >= 0
                invariants.push(case.clone().ge(isz.zero()));
                // ptr.case < def.variants.len()
                invariants
                    .push(case.clone().lt(Expr::int_constant(def.variants.len(), isz.clone())));
            } else {
                // if it's not the default case, we have to enumerate all possible values of
                // discriminants.
                let mut cases = vec![];
                let sz = match discr_t.kind() {
                    ty::Int(k) => match k {
                        IntTy::Isize => unreachable!(),
                        IntTy::I8 => 1,
                        IntTy::I16 => 2,
                        IntTy::I32 => 4,
                        IntTy::I64 => 8,
                        IntTy::I128 => 16,
                    },
                    ty::Uint(k) => match k {
                        UintTy::Usize => unreachable!(),
                        UintTy::U8 => 1,
                        UintTy::U16 => 2,
                        UintTy::U32 => 4,
                        UintTy::U64 => 8,
                        UintTy::U128 => 16,
                    },
                    _ => unreachable!(),
                };
                for (_, discr) in def.discriminants(tcx.tcx) {
                    let val = (discr.val << (128 - 8 * sz)) >> (128 - 8 * sz);
                    cases.push(case.clone().eq(tcx.codegen_const_value(
                        ConstValue::Scalar(Scalar::Int(ScalarInt { data: val, size: sz })),
                        discr_t,
                        None,
                    )));
                }
                invariants.push(fold_invariants_or(cases));
            }

            // 2. for each discriminant within the valid range, we check each field of each case
            // e.g.
            // enum Foo {
            //   C1(T1, T2),
            //   C2(T3, T4),
            // }
            //
            // we assume
            //     ptr.case == 0 ==> T1:invariant(..) && T2:invariant(..)
            // and
            //     ptr.case == 1 ==> T3:invariant(..) && T4:invariant(..)
            for (i, variant) in def.variants.iter_enumerated() {
                let idx = i.index();
                let variant_name = variant.ident.name.to_string();
                let var_struct = ptr
                    .clone()
                    .dereference()
                    .member("cases", &tcx.symbol_table)
                    .member(&variant_name, &tcx.symbol_table);
                let mut case_invariants = vec![];
                for f in &variant.fields {
                    let ft = f.ty(tcx.tcx, subst);
                    if let Some(fi) = tcx.codegen_assumption(ft) {
                        let fname = f.ident.name.to_string();
                        case_invariants.push(fi.call(vec![
                            var_struct.clone().member(&fname, &tcx.symbol_table).address_of(),
                        ]));
                    }
                }

                if !case_invariants.is_empty() {
                    invariants.push(
                        case.clone()
                            .eq(Expr::int_constant(idx, isz.clone()))
                            .implies(fold_invariants(case_invariants)),
                    );
                }
            }

            body.push(fold_invariants(invariants).ret(Location::none()));
        })
    }

    /// generates:
    ///
    /// bool t:invariant(t* ptr) {
    ///     for (int i; i < len; i++) {
    ///         if (!et:invariant(&ptr.0[i])) return false;
    ///     }
    ///     return true;
    /// }
    fn codegen_assumption_array(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        et: Ty<'tcx>,
        c: &'tcx ty::Const<'tcx>,
    ) -> Symbol {
        self.codegen_assumption_genfunc(fname, t, |tcx, ptr, body| {
            if let Some(f) = tcx.codegen_assumption(et) {
                let idx = tcx.gen_function_local_variable(2, &fname, Type::size_t());
                body.push(Stmt::decl(idx.to_expr(), Some(Type::size_t().zero()), Location::none()));
                let idxe = idx.to_expr();
                let lbody = Stmt::block(
                    vec![
                        f.call(vec![
                            tcx.codegen_idx_array(ptr.clone().dereference(), idxe.clone())
                                .address_of(),
                        ])
                        .not()
                        .if_then_else(
                            Expr::bool_false().ret(Location::none()),
                            None,
                            Location::none(),
                        ),
                    ],
                    Location::none(),
                );
                body.push(Stmt::for_loop(
                    Stmt::skip(Location::none()),
                    idxe.clone().lt(tcx.codegen_const(c, None)),
                    idxe.postincr().as_stmt(Location::none()),
                    lbody,
                    Location::none(),
                ));
                body.push(Expr::bool_true().ret(Location::none()));
            }
        })
    }

    /// the same as struct actually
    fn codegen_assumption_tuple(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        ts: ty::subst::SubstsRef<'tcx>,
    ) -> Symbol {
        self.codegen_assumption_genfunc(fname, t, |tcx, ptr, body| {
            let mut invariants = vec![];
            for (i, t) in ts.iter().enumerate() {
                let t = t.expect_ty();
                if let Some(f) = tcx.codegen_assumption(t) {
                    let field = ptr
                        .clone()
                        .dereference()
                        .member(&tuple_fld(i), &tcx.symbol_table) // x->i
                        .address_of(); // &(x->i)
                    invariants.push(f.call(vec![field]));
                }
            }
            body.push(fold_invariants(invariants).ret(Location::none()));
        })
    }

    fn codegen_assumption_genfunc<F: FnOnce(&mut GotocCtx<'tcx>, Expr, &mut Vec<Stmt>) -> ()>(
        &mut self,
        fname: &str,
        t: Ty<'tcx>,
        f: F,
    ) -> Symbol {
        //TODO this is created all out of order!
        let paramt = self.codegen_ty(t).to_pointer();
        let var = self.gen_function_local_variable(1, &fname, paramt);
        let ptr = var.clone().to_expr();
        let mut stmts = vec![];
        //let mut body = Stmt::block(vec![]);
        f(self, ptr, &mut stmts);
        let body = Stmt::block(stmts, Location::none());
        Symbol::function(
            fname,
            fname,
            Type::code(vec![var.to_function_parameter()], Type::bool()),
            Some(body),
            Location::none(),
        )
    }
}
