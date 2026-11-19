// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code used for resolve type / trait names

use crate::kani_middle::resolve::{ResolveError, resolve_path, validate_kind};
use quote::ToTokens;
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_public::CrateDef;
use rustc_public::mir::Mutability;
use rustc_public::rustc_internal;
use rustc_public::ty::{
    AdtDef, FloatTy, GenericArgKind, GenericArgs, IntTy, Region, RegionKind, RigidTy, Ty, TyKind,
    UintTy,
};
use rustc_span::def_id::LocalDefId;
use std::str::FromStr;
use strum_macros::{EnumString, IntoStaticStr};
use syn::{Expr, ExprLit, Lit, Type, TypePath};
use tracing::{debug, debug_span};

/// Attempts to resolve a type from a type expression.
pub fn resolve_ty<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    typ: &syn::Type,
) -> Result<Ty, ResolveError<'tcx>> {
    let _span = debug_span!("resolve_ty", ?typ).entered();
    debug!(?typ, ?current_module, "resolve_ty");
    let unsupported = |kind: &'static str| Err(ResolveError::UnsupportedPath { kind });
    let invalid = |kind: &'static str| {
        Err(ResolveError::InvalidPath {
            msg: format!("Expected a type, but found {kind} `{}`", typ.to_token_stream()),
        })
    };
    #[warn(non_exhaustive_omitted_patterns)]
    match typ {
        Type::Path(TypePath { qself, path }) => {
            if (*qself).is_some() {
                return unsupported("nested qualified paths");
            }
            if let Some(primitive) =
                path.get_ident().and_then(|ident| PrimitiveIdent::from_str(&ident.to_string()).ok())
            {
                Ok(primitive.into())
            } else {
                let def_id = resolve_path(tcx, current_module, path)?;
                validate_kind!(
                    tcx,
                    def_id,
                    "type",
                    DefKind::Struct | DefKind::Union | DefKind::Enum
                )?;
                let ty = rustc_internal::stable(tcx.type_of(def_id)).value;
                Ok(instantiate_path_args(tcx, current_module, path, ty))
            }
        }
        Type::Array(array) => {
            let elem_ty = resolve_ty(tcx, current_module, &array.elem)?;
            let len = parse_len(&array.len).map_err(|msg| ResolveError::InvalidPath { msg })?;
            Ty::try_new_array(elem_ty, len.try_into().unwrap()).map_err(|err| {
                ResolveError::InvalidPath { msg: format!("Cannot instantiate array. {err}") }
            })
        }
        Type::Paren(inner) => resolve_ty(tcx, current_module, &inner.elem),
        Type::Ptr(ptr) => {
            let elem_ty = resolve_ty(tcx, current_module, &ptr.elem)?;
            let mutability =
                if ptr.mutability.is_some() { Mutability::Mut } else { Mutability::Not };
            Ok(Ty::new_ptr(elem_ty, mutability))
        }
        Type::Reference(reference) => {
            let elem_ty = resolve_ty(tcx, current_module, &reference.elem)?;
            let mutability =
                if reference.mutability.is_some() { Mutability::Mut } else { Mutability::Not };
            Ok(Ty::new_ref(Region { kind: RegionKind::ReErased }, elem_ty, mutability))
        }
        Type::Slice(slice) => {
            let elem_ty = resolve_ty(tcx, current_module, &slice.elem)?;
            Ok(Ty::from_rigid_kind(RigidTy::Slice(elem_ty)))
        }
        Type::Tuple(tuple) => {
            let elems = tuple
                .elems
                .iter()
                .map(|elem| resolve_ty(tcx, current_module, elem))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Ty::new_tuple(&elems))
        }
        Type::Never(_) => Ok(Ty::from_rigid_kind(RigidTy::Never)),
        Type::BareFn(bare_fn) => {
            // Rust-ABI, non-variadic fn pointers; lifetimes are erased like everywhere
            // else in this resolver. A higher-ranked argument (`fn(&u8) -> u8`)
            // therefore matches an erased impl; if a `fn(&'static u8) -> u8` sibling
            // impl also exists, it resolves to that one instead (binding the late-bound
            // regions is a follow-up; see https://github.com/model-checking/kani/issues/4933).
            if bare_fn.variadic.is_some() {
                return unsupported("variadic bare function");
            }
            if bare_fn
                .abi
                .as_ref()
                .is_some_and(|abi| abi.name.as_ref().is_none_or(|name| name.value() != "Rust"))
            {
                return unsupported("non-Rust-ABI bare function");
            }
            let inputs = bare_fn
                .inputs
                .iter()
                .map(|arg| resolve_ty(tcx, current_module, &arg.ty))
                .collect::<Result<Vec<_>, _>>()?;
            let output = match &bare_fn.output {
                syn::ReturnType::Default => Ty::new_tuple(&[]),
                syn::ReturnType::Type(_, ty) => resolve_ty(tcx, current_module, ty)?,
            };
            let safety = if bare_fn.unsafety.is_some() {
                rustc_hir::Safety::Unsafe
            } else {
                rustc_hir::Safety::Safe
            };
            let sig = tcx.mk_fn_sig_rust_abi(
                inputs.iter().map(|ty| rustc_internal::internal(tcx, *ty)),
                rustc_internal::internal(tcx, output),
                safety,
            );
            Ok(rustc_internal::stable(rustc_middle::ty::Ty::new_fn_ptr(
                tcx,
                rustc_middle::ty::Binder::dummy(sig),
            )))
        }
        Type::Macro(_) => invalid("macro"),
        Type::Group(_) => invalid("group paths"),
        Type::ImplTrait(_) => invalid("trait impl paths"),
        Type::Infer(_) => invalid("inferred paths"),
        Type::TraitObject(_) => invalid("trait object paths"),
        Type::Verbatim(_) => unsupported("unknown paths"),
        _ => {
            unreachable!()
        }
    }
}

/// If `path`'s final segment carries angle-bracketed generic arguments, instantiate `ty`
/// (the definition's identity type, e.g. `Wrap<T>`) with those arguments resolved to
/// concrete types (e.g. `Wrap<u8>`), so trait-implementation lookups can match a concrete
/// impl. An omitted trailing parameter with a declared default is filled from the
/// default, also when the path has no generic arguments at all (`Wrapper` for
/// `struct Wrapper<T = u8>`). Returns `ty` unchanged when any argument cannot be
/// resolved or a parameter without a default is missing, preserving the previous
/// behavior for everything that resolved before.
fn instantiate_path_args<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_module: LocalDefId,
    path: &syn::Path,
    ty: Ty,
) -> Ty {
    // No generic arguments (`Wrapper`) is an empty list; parenthesized args keep `ty`.
    let syn_args: Vec<&syn::GenericArgument> = match path.segments.last().map(|seg| &seg.arguments)
    {
        Some(syn::PathArguments::AngleBracketed(args)) => args.args.iter().collect(),
        Some(syn::PathArguments::None) => Vec::new(),
        _ => return ty,
    };
    let TyKind::RigidTy(RigidTy::Adt(adt_def, identity_args)) = ty.kind() else {
        return ty;
    };
    // Resolve the user-written type arguments; lifetimes are erased below, and anything
    // else (const arguments, associated-type bindings) keeps the uninstantiated type.
    let mut user_tys = Vec::new();
    for arg in syn_args {
        match arg {
            syn::GenericArgument::Type(syn_ty) => match resolve_ty(tcx, current_module, syn_ty) {
                Ok(t) => user_tys.push(t),
                Err(_) => return ty,
            },
            syn::GenericArgument::Lifetime(_) => {}
            _ => return ty,
        }
    }
    // Substitute the definition's type parameters in declaration order; erase lifetime
    // parameters; fill an omitted trailing parameter that has a declared default from
    // that default, instantiated with the arguments so far — what rustc does for omitted
    // arguments. A missing parameter without a default keeps the uninstantiated type.
    let mut user_iter = user_tys.into_iter();
    let mut new_args = Vec::new();
    for (param_index, arg) in identity_args.0.iter().enumerate() {
        match arg {
            GenericArgKind::Type(_) => {
                let filled = user_iter
                    .next()
                    .or_else(|| default_type_arg(tcx, &adt_def, param_index, &new_args));
                match filled {
                    Some(t) => new_args.push(GenericArgKind::Type(t)),
                    None => return ty,
                }
            }
            GenericArgKind::Lifetime(_) => {
                new_args.push(GenericArgKind::Lifetime(Region { kind: RegionKind::ReErased }))
            }
            GenericArgKind::Const(_) => return ty,
        }
    }
    if user_iter.next().is_some() {
        return ty;
    }
    Ty::from_rigid_kind(RigidTy::Adt(adt_def, GenericArgs(new_args)))
}

/// The declared default of `adt_def`'s `param_index`-th generic parameter, instantiated
/// with the arguments already substituted before it — how rustc fills an omitted trailing
/// argument (a default may only reference earlier parameters). `None` when the parameter
/// has no default or the default is not a type; the caller then keeps the uninstantiated
/// type.
fn default_type_arg<'tcx>(
    tcx: TyCtxt<'tcx>,
    adt_def: &AdtDef,
    param_index: usize,
    args_so_far: &[GenericArgKind],
) -> Option<Ty> {
    let def_id = rustc_internal::internal(tcx, adt_def.def_id());
    let default = tcx.generics_of(def_id).own_params.get(param_index)?.default_value(tcx)?;
    let internal_args: Vec<rustc_middle::ty::GenericArg<'tcx>> = args_so_far
        .iter()
        .map(|arg| match arg {
            GenericArgKind::Type(t) => Some(rustc_internal::internal(tcx, *t).into()),
            GenericArgKind::Lifetime(_) => Some(tcx.lifetimes.re_erased.into()),
            GenericArgKind::Const(_) => None,
        })
        .collect::<Option<_>>()?;
    let filled = default.instantiate(tcx, &internal_args[..]).skip_normalization().as_type()?;
    Some(rustc_internal::stable(filled))
}

/// Enumeration of existing primitive types that are not parametric.
#[derive(Copy, Clone, Debug, Eq, PartialEq, IntoStaticStr, EnumString)]
#[strum(serialize_all = "lowercase")]
pub(super) enum PrimitiveIdent {
    Bool,
    Char,
    F16,
    F32,
    F64,
    F128,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    Str,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
}

/// Convert a primitive ident into a primitive `Ty`.
impl From<PrimitiveIdent> for Ty {
    fn from(value: PrimitiveIdent) -> Self {
        match value {
            PrimitiveIdent::Bool => Ty::bool_ty(),
            PrimitiveIdent::Char => Ty::from_rigid_kind(RigidTy::Char),
            PrimitiveIdent::F16 => Ty::from_rigid_kind(RigidTy::Float(FloatTy::F16)),
            PrimitiveIdent::F32 => Ty::from_rigid_kind(RigidTy::Float(FloatTy::F32)),
            PrimitiveIdent::F64 => Ty::from_rigid_kind(RigidTy::Float(FloatTy::F64)),
            PrimitiveIdent::F128 => Ty::from_rigid_kind(RigidTy::Float(FloatTy::F128)),
            PrimitiveIdent::I8 => Ty::signed_ty(IntTy::I8),
            PrimitiveIdent::I16 => Ty::signed_ty(IntTy::I16),
            PrimitiveIdent::I32 => Ty::signed_ty(IntTy::I32),
            PrimitiveIdent::I64 => Ty::signed_ty(IntTy::I64),
            PrimitiveIdent::I128 => Ty::signed_ty(IntTy::I128),
            PrimitiveIdent::Isize => Ty::signed_ty(IntTy::Isize),
            PrimitiveIdent::Str => Ty::from_rigid_kind(RigidTy::Str),
            PrimitiveIdent::U8 => Ty::unsigned_ty(UintTy::U8),
            PrimitiveIdent::U16 => Ty::unsigned_ty(UintTy::U16),
            PrimitiveIdent::U32 => Ty::unsigned_ty(UintTy::U32),
            PrimitiveIdent::U64 => Ty::unsigned_ty(UintTy::U64),
            PrimitiveIdent::U128 => Ty::unsigned_ty(UintTy::U128),
            PrimitiveIdent::Usize => Ty::unsigned_ty(UintTy::Usize),
        }
    }
}

/// Checks if a Path segment represents a primitive.
///
/// Note that this function will return false for expressions that cannot be parsed as a type.
pub(super) fn is_primitive<T>(path: &T) -> bool
where
    T: ToTokens,
{
    let token = path.to_token_stream();
    let Ok(typ) = syn::parse2(token) else { return false };
    is_type_primitive(&typ)
}

/// Checks if a type is a primitive including composite ones.
pub(super) fn is_type_primitive(typ: &syn::Type) -> bool {
    #[warn(non_exhaustive_omitted_patterns)]
    match typ {
        Type::Array(_)
        | Type::Ptr(_)
        | Type::Reference(_)
        | Type::Slice(_)
        | Type::Never(_)
        | Type::Tuple(_) => true,
        Type::Path(TypePath { qself: Some(qself), path }) => {
            path.segments.is_empty() && is_type_primitive(&qself.ty)
        }
        Type::Path(TypePath { qself: None, path }) => path
            .get_ident()
            .is_some_and(|ident| PrimitiveIdent::from_str(&ident.to_string()).is_ok()),
        Type::BareFn(_)
        | Type::Group(_)
        | Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Paren(_)
        | Type::TraitObject(_)
        | Type::Verbatim(_) => false,
        _ => {
            unreachable!()
        }
    }
}

/// Parse the length of the array.
/// We currently only support a constant length.
fn parse_len(len: &Expr) -> Result<usize, String> {
    if let Expr::Lit(ExprLit { lit: Lit::Int(lit), .. }) = len
        && matches!(lit.suffix(), "" | "usize")
        && let Ok(val) = usize::from_str(lit.base10_digits())
    {
        return Ok(val);
    }
    Err(format!("Expected a `usize` constant, but found `{}`", len.to_token_stream()))
}
