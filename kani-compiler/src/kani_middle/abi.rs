// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for handling type abi information.

use stable_mir::abi::{FieldsShape, LayoutShape};
use stable_mir::ty::{RigidTy, Ty, TyKind, UintTy};
use tracing::debug;

/// A struct to encapsulate the layout information for a given type
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutOf {
    ty: Ty,
    layout: LayoutShape,
}

#[allow(dead_code)] // TODO: Remove this in https://github.com/model-checking/kani/pull/3687
impl LayoutOf {
    /// Create the layout structure for the given type
    pub fn new(ty: Ty) -> LayoutOf {
        LayoutOf { ty, layout: ty.layout().unwrap().shape() }
    }

    /// Return whether the type is sized.
    pub fn is_sized(&self) -> bool {
        self.layout.is_sized()
    }

    /// Return whether the type is unsized and its tail is a foreign item.
    ///
    /// This will also return `true` if the type is foreign.
    pub fn has_foreign_tail(&self) -> bool {
        self.unsized_tail()
            .map_or(false, |t| matches!(t.kind(), TyKind::RigidTy(RigidTy::Foreign(_))))
    }

    /// Return whether the type is unsized and its tail is a trait object.
    pub fn has_trait_tail(&self) -> bool {
        self.unsized_tail().map_or(false, |t| t.kind().is_trait())
    }

    /// Return whether the type is unsized and its tail is a slice.
    #[allow(dead_code)]
    pub fn has_slice_tail(&self) -> bool {
        self.unsized_tail().map_or(false, |tail| {
            let kind = tail.kind();
            kind.is_slice() | kind.is_str()
        })
    }

    /// Return the unsized tail of the type if this is an unsized type.
    ///
    /// For foreign types, return None.
    /// For unsized types, this should return either a slice, a string slice, a dynamic type.
    /// For other types, this function will return `None`.
    pub fn unsized_tail(&self) -> Option<Ty> {
        if self.layout.is_unsized() {
            match self.ty.kind().rigid().unwrap() {
                RigidTy::Slice(..) | RigidTy::Dynamic(..) | RigidTy::Str => Some(self.ty),
                RigidTy::Adt(..) | RigidTy::Tuple(..) => {
                    // Recurse the tail field type until we find the unsized tail.
                    self.last_field_layout().unsized_tail()
                }
                RigidTy::Foreign(_) => Some(self.ty),
                _ => unreachable!("Expected unsized type but found `{}`", self.ty),
            }
        } else {
            None
        }
    }

    /// Return the type of the elements of the array or slice at the unsized tail of this type.
    ///
    /// For sized types and trait unsized type, this function will return `None`.
    pub fn unsized_tail_elem_ty(&self) -> Option<Ty> {
        self.unsized_tail().and_then(|tail| match tail.kind().rigid().unwrap() {
            RigidTy::Slice(elem_ty) => Some(*elem_ty),
            // String slices have the same layout as slices of u8.
            // https://doc.rust-lang.org/reference/type-layout.html#str-layout
            RigidTy::Str => Some(Ty::unsigned_ty(UintTy::U8)),
            _ => None,
        })
    }

    /// Return the size of the sized portion of this type.
    ///
    /// For a sized type, this function will return the size of the type.
    /// For an unsized type, this function will return the size of the sized portion including
    /// any padding bytes that lead to the unsized field.
    /// I.e.: the size of the type, excluding the trailing unsized portion.
    ///
    /// For example, this function will return 2 as the sized portion of `*const (u8,  [u16])`:
    pub fn size_of_head(&self) -> usize {
        if self.is_sized() {
            self.layout.size.bytes()
        } else {
            match self.ty.kind().rigid().unwrap() {
                RigidTy::Slice(_) | RigidTy::Str | RigidTy::Dynamic(..) | RigidTy::Foreign(..) => 0,
                RigidTy::Adt(..) | RigidTy::Tuple(..) => {
                    let fields_sorted = self.layout.fields.fields_by_offset_order();
                    let last = *fields_sorted.last().unwrap();
                    let FieldsShape::Arbitrary { ref offsets } = self.layout.fields else {
                        unreachable!()
                    };

                    // We compute the size of the sized portion by taking the position of the
                    // last element + the sized portion of that element.
                    let unsized_offset_unadjusted = offsets[last].bytes();
                    debug!(ty=?self.ty, ?unsized_offset_unadjusted, "size_of_sized_portion");
                    unsized_offset_unadjusted + self.last_field_layout().size_of_head()
                }
                _ => {
                    unreachable!("Expected sized type, but found: `{}`", self.ty)
                }
            }
        }
    }

    /// Return the alignment of the fields that are sized from the head of the object.
    ///
    /// For a sized type, this function will return the alignment of the type.
    ///
    /// For an unsized type, this function will return the alignment of the sized portion.
    /// The alignment of the type will be the maximum of the alignment of the sized portion
    /// and the alignment of the unsized portion.
    ///
    /// If there's no sized portion, this function will return the minimum alignment (i.e.: 1).
    pub fn align_of_head(&self) -> usize {
        if self.is_sized() {
            self.layout.abi_align.try_into().unwrap()
        } else {
            match self.ty.kind().rigid().unwrap() {
                RigidTy::Slice(_) | RigidTy::Str | RigidTy::Dynamic(..) | RigidTy::Foreign(..) => 1,
                RigidTy::Adt(..) | RigidTy::Tuple(..) => {
                    // We have to recurse and get the maximum alignment of all sized portions.
                    let field_layout = self.last_field_layout();
                    field_layout.align_of_head().max(self.layout.abi_align.try_into().unwrap())
                }
                _ => {
                    unreachable!("Expected sized type, but found: `{}`", self.ty);
                }
            }
        }
    }

    /// Return the size of the type if it's known at compilation type.
    pub fn size_of(&self) -> Option<usize> {
        if self.is_sized() { Some(self.layout.size.bytes()) } else { None }
    }

    /// Return the alignment of the type if it's know at compilation time.
    ///
    /// The alignment is known at compilation type for sized types and types with slice tail.
    ///
    /// Note: We assume u64 and usize are the same since Kani is only supported in 64bits machines.
    /// Add a configuration in case we ever decide to port this to a 32-bits machine.
    #[cfg(target_pointer_width = "64")]
    pub fn align_of(&self) -> Option<usize> {
        if self.is_sized() || self.has_slice_tail() {
            self.layout.abi_align.try_into().ok()
        } else {
            None
        }
    }

    /// Return the layout of the last field of the type.
    ///
    /// This function is used to get the layout of the last field of an unsized type.
    /// For example, if we have `*const (u8, [u16])`, the last field is `[u16]`.
    /// This function will return the layout of `[u16]`.
    ///
    /// If the type is not a struct, an enum, or a tuple, with at least one field, this function
    /// will panic.
    fn last_field_layout(&self) -> LayoutOf {
        match self.ty.kind().rigid().unwrap() {
            RigidTy::Adt(adt_def, adt_args) => {
                let fields = adt_def.variants_iter().next().unwrap().fields();
                let fields_sorted = self.layout.fields.fields_by_offset_order();
                let last_field_idx = *fields_sorted.last().unwrap();
                LayoutOf::new(fields[last_field_idx].ty_with_args(adt_args))
            }
            RigidTy::Tuple(tys) => {
                // For tuples, the unsized field is currently the last declared.
                // To be on the safe side, we still get the sorted list by offset order.
                let fields_sorted = self.layout.fields.fields_by_offset_order();
                let last_field_idx = *fields_sorted.last().unwrap();
                let last_ty = tys[last_field_idx];
                LayoutOf::new(last_ty)
            }
            _ => {
                unreachable!("Expected struct, enum or tuple. Found: `{}`", self.ty);
            }
        }
    }
}
