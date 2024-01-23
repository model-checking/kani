// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains methods that help us process coercions.
//! There are many types of coercions in rust, they are described in the
//! [RFC 401 Coercions](https://rust-lang.github.io/rfcs/0401-coercions.html).
//!
//! The more complicated coercions are DST Coercions (aka Unsized Coercions). These coercions
//! allow rust to create references to dynamically sized types (such as Traits and Slices) by
//! casting concrete sized types. Unsized coercions can also be used to cast unsized to unsized
//! types. These casts work not only on the top of references, but it also handle
//! references inside structures, allowing the unsized coercions of smart pointers. The
//! definition of custom coercions for smart pointers can be found in the
//! [RFC 982 DST Coercion](https://rust-lang.github.io/rfcs/0982-dst-coercion.html).

use rustc_hir::lang_items::LangItem;
use rustc_middle::traits::{ImplSource, ImplSourceUserDefinedData};
use rustc_middle::ty::adjustment::CustomCoerceUnsized;
use rustc_middle::ty::{ParamEnv, Ty, TyCtxt};
use rustc_middle::ty::{TraitRef, TypeAndMut};
use rustc_smir::rustc_internal;
use stable_mir::ty::{RigidTy, Ty as TyStable, TyKind};
use stable_mir::Symbol;
use tracing::trace;

/// Given an unsized coercion (e.g. from `&u8` to `&dyn Debug`), extract the pair of
/// corresponding base types `T`, `U` (e.g. `u8`, `dyn Debug`), where the source base type `T` must
/// implement `Unsize<U>` and `U` is either a trait or slice.
///
/// For more details, please refer to:
/// <https://doc.rust-lang.org/reference/type-coercions.html#unsized-coercions>
///
/// This is used to determine the vtable implementation that must be tracked by the fat pointer.
///
/// For example, if `&u8` is being converted to `&dyn Debug`, this method would return:
/// `(u8, dyn Debug)`.
///
/// There are a few interesting cases (references / pointers are handled the same way):
/// 1. Coercion between `&T` to `&U`.
///    - This is the base case.
///    - In this case, we extract the types that are pointed to.
/// 2. Coercion between smart pointers like `NonNull<T>` to `NonNull<U>`.
///    - Smart pointers implement the `CoerceUnsize` trait.
///    - Use CustomCoerceUnsized information to traverse the smart pointer structure and find the
///      underlying pointer.
///    - Use base case to extract `T` and `U`.
/// 3. Coercion between a pointer to a structure whose tail is being coerced.
///
/// E.g.: A user may want to define a type like:
/// ```
/// struct Message<T> {
///     header: &str,
///     content: T,
/// }
/// ```
///    They may want to abstract only the content of a message. So one could coerce a
///   `&Message<String>` into a `&Message<dyn Display>`. In this case, this would:
///    - Apply base case to extract the pair `(Message<T>, Message<U>)`.
///    - Extract the tail element of the struct which are of type `T` and `U`, respectively.
/// 4. Coercion between smart pointers of wrapper structs.
///    - Apply the logic from item 2 then item 3.
pub fn extract_unsize_casting_stable(
    tcx: TyCtxt,
    src_ty: TyStable,
    dst_ty: TyStable,
) -> CoercionBaseStable {
    let CoercionBase { src_ty, dst_ty } = extract_unsize_casting(
        tcx,
        rustc_internal::internal(tcx, src_ty),
        rustc_internal::internal(tcx, dst_ty),
    );
    CoercionBaseStable {
        src_ty: rustc_internal::stable(src_ty),
        dst_ty: rustc_internal::stable(dst_ty),
    }
}

pub fn extract_unsize_casting<'tcx>(
    tcx: TyCtxt<'tcx>,
    src_ty: Ty<'tcx>,
    dst_ty: Ty<'tcx>,
) -> CoercionBase<'tcx> {
    trace!(?src_ty, ?dst_ty, "extract_unsize_casting");
    // Iterate over the pointer structure to find the builtin pointer that will store the metadata.
    let coerce_info = CoerceUnsizedIterator::new(
        tcx,
        rustc_internal::stable(src_ty),
        rustc_internal::stable(dst_ty),
    )
    .last()
    .unwrap();
    // Extract the pointee type that is being coerced.
    let src_pointee_ty = extract_pointee(tcx, coerce_info.src_ty).expect(&format!(
        "Expected source to be a pointer. Found {:?} instead",
        coerce_info.src_ty
    ));
    let dst_pointee_ty = extract_pointee(tcx, coerce_info.dst_ty).expect(&format!(
        "Expected destination to be a pointer. Found {:?} instead",
        coerce_info.dst_ty
    ));
    // Find the tail of the coercion that determines the type of metadata to be stored.
    let (src_base_ty, dst_base_ty) = tcx.struct_lockstep_tails_erasing_lifetimes(
        src_pointee_ty,
        dst_pointee_ty,
        ParamEnv::reveal_all(),
    );
    trace!(?src_base_ty, ?dst_base_ty, "extract_unsize_casting result");
    assert!(
        dst_base_ty.is_trait() || dst_base_ty.is_slice(),
        "Expected trait or slice as destination of unsized cast, but found {dst_base_ty:?}"
    );
    CoercionBase { src_ty: src_base_ty, dst_ty: dst_base_ty }
}

/// This structure represents the base of a coercion.
///
/// This base is used to determine the information that will be stored in the metadata.
/// E.g.: In order to convert an `Rc<String>` into an `Rc<dyn Debug>`, we need to generate a
/// vtable that represents the `impl Debug for String`. So this type will carry the `String` type
/// as the `src_ty` and the `dyn Debug` trait as `dst_ty`.
#[derive(Debug)]
pub struct CoercionBase<'tcx> {
    pub src_ty: Ty<'tcx>,
    pub dst_ty: Ty<'tcx>,
}

#[derive(Debug)]
pub struct CoercionBaseStable {
    pub src_ty: TyStable,
    pub dst_ty: TyStable,
}
/// Iterates over the coercion path of a structure that implements `CoerceUnsized<T>` trait.
/// The `CoerceUnsized<T>` trait indicates that this is a pointer or a wrapper for one, where
/// unsizing can be performed on the pointee. More details:
/// <https://doc.rust-lang.org/std/ops/trait.CoerceUnsized.html>
///
/// Given an unsized coercion between `impl CoerceUnsized<T>` to `impl CoerceUnsized<U>` where
/// `T` is sized and `U` is unsized, this iterator will walk over the fields that lead to a
/// pointer to `T`, which shall be converted from a thin pointer to a fat pointer.
///
/// Each iteration will also include an optional name of the field that differs from the current
/// pair of types.
///
/// The first element of the iteration will always be the starting types.
/// The last element of the iteration will always be pointers to `T` and `U`.
/// After unsized element has been found, the iterator will return `None`.
pub struct CoerceUnsizedIterator<'tcx> {
    tcx: TyCtxt<'tcx>,
    src_ty: Option<TyStable>,
    dst_ty: Option<TyStable>,
}

/// Represent the information about a coercion.
#[derive(Debug, Clone, PartialEq)]
pub struct CoerceUnsizedInfo {
    /// The name of the field from the current types that differs between each other.
    pub field: Option<Symbol>,
    /// The type being coerced.
    pub src_ty: TyStable,
    /// The type that is the result of the coercion.
    pub dst_ty: TyStable,
}

impl<'tcx> CoerceUnsizedIterator<'tcx> {
    pub fn new(
        tcx: TyCtxt<'tcx>,
        src_ty: TyStable,
        dst_ty: TyStable,
    ) -> CoerceUnsizedIterator<'tcx> {
        CoerceUnsizedIterator { tcx, src_ty: Some(src_ty), dst_ty: Some(dst_ty) }
    }
}

/// Iterate over the coercion path. At each iteration, it returns the name of the field that must
/// be coerced, as well as the current source and the destination.
/// E.g.: The first iteration of casting `NonNull<String>` -> `NonNull<&dyn Debug>` will return
/// ```rust,ignore
/// CoerceUnsizedInfo {
///    field: Some("ptr"),
///    src_ty, // NonNull<String>
///    dst_ty  // NonNull<&dyn Debug>
/// }
/// ```
/// while the last iteration will return:
/// ```rust,ignore
/// CoerceUnsizedInfo {
///   field: None,
///   src_ty: Ty, // *const String
///   dst_ty: Ty, // *const &dyn Debug
/// }
/// ```
impl<'tcx> Iterator for CoerceUnsizedIterator<'tcx> {
    type Item = CoerceUnsizedInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if self.src_ty.is_none() {
            assert_eq!(self.dst_ty, None, "Expected no dst type.");
            return None;
        }

        // Extract the pointee types from pointers (including smart pointers) that form the base of
        // the conversion.
        let src_ty = self.src_ty.take().unwrap();
        let dst_ty = self.dst_ty.take().unwrap();
        let field = match (src_ty.kind(), dst_ty.kind()) {
            (
                TyKind::RigidTy(RigidTy::Adt(src_def, ref src_args)),
                TyKind::RigidTy(RigidTy::Adt(dst_def, ref dst_args)),
            ) => {
                // Handle smart pointers by using CustomCoerceUnsized to find the field being
                // coerced.
                assert_eq!(src_def, dst_def);
                let src_fields = &src_def.variants_iter().next().unwrap().fields();
                let dst_fields = &dst_def.variants_iter().next().unwrap().fields();
                assert_eq!(src_fields.len(), dst_fields.len());

                let CustomCoerceUnsized::Struct(coerce_index) = custom_coerce_unsize_info(
                    self.tcx,
                    rustc_internal::internal(self.tcx, src_ty),
                    rustc_internal::internal(self.tcx, dst_ty),
                );
                let coerce_index = coerce_index.as_usize();
                assert!(coerce_index < src_fields.len());

                self.src_ty = Some(src_fields[coerce_index].ty_with_args(&src_args));
                self.dst_ty = Some(dst_fields[coerce_index].ty_with_args(&dst_args));
                Some(src_fields[coerce_index].name.clone())
            }
            _ => {
                // Base case is always a pointer (Box, raw_pointer or reference).
                assert!(
                    extract_pointee(self.tcx, src_ty).is_some(),
                    "Expected a pointer, but found {src_ty:?}"
                );
                None
            }
        };
        Some(CoerceUnsizedInfo { field, src_ty, dst_ty })
    }
}

/// Get information about an unsized coercion.
/// This code was extracted from `rustc_monomorphize` crate.
/// <https://github.com/rust-lang/rust/blob/4891d57f7aab37b5d6a84f2901c0bb8903111d53/compiler/rustc_monomorphize/src/lib.rs#L25-L46>
fn custom_coerce_unsize_info<'tcx>(
    tcx: TyCtxt<'tcx>,
    source_ty: Ty<'tcx>,
    target_ty: Ty<'tcx>,
) -> CustomCoerceUnsized {
    let def_id = tcx.require_lang_item(LangItem::CoerceUnsized, None);

    let trait_ref = TraitRef::new(tcx, def_id, tcx.mk_args_trait(source_ty, [target_ty.into()]));

    match tcx.codegen_select_candidate((ParamEnv::reveal_all(), trait_ref)) {
        Ok(ImplSource::UserDefined(ImplSourceUserDefinedData { impl_def_id, .. })) => {
            tcx.coerce_unsized_info(impl_def_id).custom_kind.unwrap()
        }
        impl_source => {
            unreachable!("invalid `CoerceUnsized` impl_source: {:?}", impl_source);
        }
    }
}

/// Extract pointee type from builtin pointer types.
fn extract_pointee(tcx: TyCtxt<'_>, typ: TyStable) -> Option<Ty<'_>> {
    rustc_internal::internal(tcx, typ).builtin_deref(true).map(|TypeAndMut { ty, .. }| ty)
}
