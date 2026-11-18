// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Utility functions that help calculate type layout.

use std::fmt::Display;

use rustc_public::CrateDefType;
use rustc_public::{
    abi::{FieldsShape, Scalar, TagEncoding, ValueRepr, VariantsShape},
    target::{MachineInfo, MachineSize},
    ty::{AdtKind, GenericArgKind, RigidTy, Ty, TyKind, UintTy, VariantIdx},
};
use rustc_public_bridge::IndexedVal;

/// What the bytes of a [DataBytes] chunk hold.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum ByteKind {
    /// Bytes of a value of a non-union type: initialized whenever a value of the enclosing type is.
    Data,
    /// Bytes of a union nested in the enclosing type. Which of them are initialized depends on
    /// which field was last written, not on the enclosing type: a union may hold uninitialized
    /// bytes in a valid value (`MaybeUninit::uninit()`).
    Union,
}

/// Represents a chunk of data bytes in a data structure.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct DataBytes {
    /// Offset in bytes.
    offset: usize,
    /// Size of this data chunk.
    size: MachineSize,
    /// Whether the chunk holds data bytes or the bytes of a nested union.
    kind: ByteKind,
}

impl DataBytes {
    fn data(offset: usize, size: MachineSize) -> Self {
        DataBytes { offset, size, kind: ByteKind::Data }
    }
}

/// Bytewise mask, representing which bytes of a type are data and which are padding. Here, `false`
/// represents padding bytes and `true` represents data bytes.
type Layout = Vec<bool>;

/// Create a byte-wise mask of the chunks of `kind`.
fn generate_mask(size_in_bytes: usize, chunks: &[DataBytes], kind: ByteKind) -> Vec<bool> {
    let mut mask = vec![false; size_in_bytes];
    for chunk in chunks.iter().filter(|chunk| chunk.kind == kind) {
        for item in mask.iter_mut().skip(chunk.offset).take(chunk.size.bytes()) {
            *item = true;
        }
    }
    mask
}

/// Create a byte-wise mask from known chunks of data bytes. Union bytes are not data bytes.
fn generate_byte_mask(size_in_bytes: usize, data_chunks: Vec<DataBytes>) -> Vec<bool> {
    generate_mask(size_in_bytes, &data_chunks, ByteKind::Data)
}

/// The byte-wise mask of the bytes that belong to nested unions; empty if there are none.
fn generate_union_mask(size_in_bytes: usize, data_chunks: &[DataBytes]) -> UnionMask {
    if data_chunks.iter().any(|chunk| chunk.kind == ByteKind::Union && chunk.size.bytes() > 0) {
        generate_mask(size_in_bytes, data_chunks, ByteKind::Union)
    } else {
        vec![]
    }
}

/// Bytewise mask of the bytes that belong to unions nested in a type (`true`), or an empty vector
/// if the type contains no union.
///
/// Such bytes are not data bytes: a check of the enclosing value does not require them to be
/// initialized, since a union may legitimately hold uninitialized bytes. When the enclosing
/// value is marked initialized without being written (e.g. a pointer to it is created from a
/// reference), their state is left unchanged; when it is written, their state is the source's.
pub type UnionMask = Vec<bool>;

// Depending on whether the type is statically or dynamically sized,
// the layout of the element or the layout of the actual type is returned.
pub enum PointeeLayout {
    /// Layout of sized objects, with the bytes of unions nested in them.
    Sized { layout: Layout, union_mask: UnionMask },
    /// Layout of slices, *const/mut str is included in this case and treated as *const/mut [u8].
    Slice { element_layout: Layout, element_union_mask: UnionMask },
    /// Layout of unions, which are shared storage for multiple fields of potentially different layouts.
    Union { field_layouts: Vec<Layout> },
    /// Trait objects have an arbitrary layout.
    TraitObject,
}

impl PointeeLayout {
    /// Returns the size of the layout, if available.
    pub fn maybe_size(&self) -> Option<usize> {
        match self {
            PointeeLayout::Sized { layout, .. } => Some(layout.len()),
            PointeeLayout::Slice { element_layout, .. } => Some(element_layout.len()),
            PointeeLayout::Union { field_layouts } => {
                Some(field_layouts.iter().map(|field_layout| field_layout.len()).max().unwrap())
            }
            PointeeLayout::TraitObject => None,
        }
    }
}

pub struct PointeeInfo {
    pointee_ty: Ty,
    layout: PointeeLayout,
}

/// Different layout computation errors that could arise from the currently unsupported constructs.
pub enum LayoutComputationError {
    UnknownUnsizedLayout(Ty),
    EnumWithNicheEncoding(Ty),
    EnumWithMultiplePaddingVariants(Ty),
    UnsupportedType(Ty),
}

impl Display for LayoutComputationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutComputationError::UnknownUnsizedLayout(ty) => {
                write!(f, "Cannot determine layout for an unsized type {ty}")
            }
            LayoutComputationError::EnumWithNicheEncoding(ty) => {
                write!(f, "Cannot determine layout for an Enum with niche encoding of type {ty}")
            }
            LayoutComputationError::EnumWithMultiplePaddingVariants(ty) => write!(
                f,
                "Cannot determine layout for an Enum of type {ty}, as it has multiple variants that have different padding."
            ),
            LayoutComputationError::UnsupportedType(ty) => {
                write!(f, "Cannot determine layout for an unsupported type {ty}.")
            }
        }
    }
}

impl PointeeInfo {
    /// The bytes of a sized `pointee_ty` that `mask` marks, e.g. those a check must cover.
    pub fn with_mask(pointee_ty: Ty, mask: Vec<bool>) -> Self {
        PointeeInfo {
            pointee_ty,
            layout: PointeeLayout::Sized { layout: mask, union_mask: vec![] },
        }
    }

    pub fn from_ty(ty: Ty) -> Result<Self, LayoutComputationError> {
        match ty.kind() {
            TyKind::RigidTy(rigid_ty) => match rigid_ty {
                RigidTy::Adt(adt_def, args) if adt_def.kind() == AdtKind::Union => {
                    assert!(adt_def.variants().len() == 1);
                    let fields: Result<_, _> = adt_def
                        .variant(VariantIdx::to_val(0))
                        .unwrap()
                        .fields()
                        .into_iter()
                        .map(|field_def| {
                            let ty = field_def.ty_with_args(&args);
                            let size_in_bytes = ty.layout().unwrap().shape().size.bytes();
                            data_bytes_for_ty(&MachineInfo::target(), ty, 0)
                                .map(|data_chunks| generate_byte_mask(size_in_bytes, data_chunks))
                        })
                        .collect();
                    Ok(PointeeInfo {
                        pointee_ty: ty,
                        layout: PointeeLayout::Union { field_layouts: fields? },
                    })
                }
                RigidTy::Str => {
                    let slicee_ty = Ty::unsigned_ty(UintTy::U8);
                    let size_in_bytes = slicee_ty.layout().unwrap().shape().size.bytes();
                    let data_chunks = data_bytes_for_ty(&MachineInfo::target(), slicee_ty, 0)?;
                    let layout = PointeeLayout::Slice {
                        element_union_mask: generate_union_mask(size_in_bytes, &data_chunks),
                        element_layout: generate_byte_mask(size_in_bytes, data_chunks),
                    };
                    Ok(PointeeInfo { pointee_ty: ty, layout })
                }
                RigidTy::Slice(slicee_ty) => {
                    let size_in_bytes = slicee_ty.layout().unwrap().shape().size.bytes();
                    let data_chunks = data_bytes_for_ty(&MachineInfo::target(), slicee_ty, 0)?;
                    let layout = PointeeLayout::Slice {
                        element_union_mask: generate_union_mask(size_in_bytes, &data_chunks),
                        element_layout: generate_byte_mask(size_in_bytes, data_chunks),
                    };
                    Ok(PointeeInfo { pointee_ty: ty, layout })
                }
                RigidTy::Dynamic(..) => {
                    Ok(PointeeInfo { pointee_ty: ty, layout: PointeeLayout::TraitObject })
                }
                _ => {
                    if ty.layout().unwrap().shape().is_sized() {
                        let size_in_bytes = ty.layout().unwrap().shape().size.bytes();
                        let data_chunks = data_bytes_for_ty(&MachineInfo::target(), ty, 0)?;
                        let layout = PointeeLayout::Sized {
                            union_mask: generate_union_mask(size_in_bytes, &data_chunks),
                            layout: generate_byte_mask(size_in_bytes, data_chunks),
                        };
                        Ok(PointeeInfo { pointee_ty: ty, layout })
                    } else {
                        Err(LayoutComputationError::UnknownUnsizedLayout(ty))
                    }
                }
            },
            TyKind::Alias(..) | TyKind::Param(..) | TyKind::Bound(..) => {
                unreachable!("Should only encounter monomorphized types at this point.")
            }
        }
    }

    pub fn ty(&self) -> &Ty {
        &self.pointee_ty
    }

    pub fn layout(&self) -> &PointeeLayout {
        &self.layout
    }
}

/// Retrieve a set of data bytes with offsets for a type.
fn data_bytes_for_ty(
    machine_info: &MachineInfo,
    ty: Ty,
    current_offset: usize,
) -> Result<Vec<DataBytes>, LayoutComputationError> {
    let layout = ty.layout().unwrap().shape();

    match layout.fields {
        // A zero-sized primitive, e.g. the never type `!`, has no data bytes to track (its
        // layout is not a scalar, so it must not reach the scalar case below).
        FieldsShape::Primitive if layout.size.bytes() == 0 => Ok(vec![]),
        FieldsShape::Primitive => Ok(vec![match layout.value_repr {
            ValueRepr::Scalar(Scalar::Initialized { value, .. }) => {
                DataBytes::data(current_offset, value.size(machine_info))
            }
            _ => unreachable!("FieldsShape::Primitive with a different ABI than ValueRepr::Scalar"),
        }]),
        FieldsShape::Array { stride, count } if count > 0 => {
            let TyKind::RigidTy(RigidTy::Array(elem_ty, _)) = ty.kind() else { unreachable!() };
            let elem_data_bytes = data_bytes_for_ty(machine_info, elem_ty, current_offset)?;
            let mut result = vec![];
            if !elem_data_bytes.is_empty() {
                for idx in 0..count {
                    let idx: usize = idx.try_into().unwrap();
                    let elem_offset = idx * stride.bytes();
                    let mut next_data_bytes = elem_data_bytes
                        .iter()
                        .cloned()
                        .map(|mut req| {
                            req.offset += elem_offset;
                            req
                        })
                        .collect::<Vec<_>>();
                    result.append(&mut next_data_bytes)
                }
            }
            Ok(result)
        }
        FieldsShape::Arbitrary { ref offsets } => {
            match ty.kind().rigid().unwrap_or_else(|| panic!("unexpected type: {ty:?}")) {
                RigidTy::Adt(def, args) => {
                    match def.kind() {
                        AdtKind::Enum => {
                            // Support basic enumeration forms
                            let ty_variants = def.variants();
                            match layout.variants {
                                VariantsShape::Empty => Ok(vec![]),
                                VariantsShape::Single { index } => {
                                    // Only one variant is reachable. This behaves like a struct.
                                    let fields = ty_variants[index.to_index()].fields();
                                    let mut fields_data_bytes = vec![];
                                    for idx in layout.fields.fields_by_offset_order() {
                                        let field_offset = offsets[idx].bytes();
                                        let field_ty = fields[idx].ty_with_args(args);
                                        fields_data_bytes.append(&mut data_bytes_for_ty(
                                            machine_info,
                                            field_ty,
                                            field_offset + current_offset,
                                        )?);
                                    }
                                    Ok(fields_data_bytes)
                                }
                                VariantsShape::Multiple {
                                    tag_encoding: TagEncoding::Niche { .. },
                                    ..
                                } => match layout.value_repr {
                                    // The value is a single initialized scalar in every variant,
                                    // e.g. `Option<NonZero<usize>>` or `Option<&T>`: all its
                                    // bytes are data bytes.
                                    ValueRepr::Scalar(Scalar::Initialized { value, .. }) => {
                                        Ok(vec![DataBytes::data(
                                            current_offset,
                                            value.size(machine_info),
                                        )])
                                    }
                                    _ => Err(LayoutComputationError::EnumWithNicheEncoding(ty)),
                                },
                                VariantsShape::Multiple { variants, tag, .. } => {
                                    // Retrieve data bytes for the tag.
                                    let tag_size = match tag {
                                        Scalar::Initialized { value, .. } => {
                                            value.size(machine_info)
                                        }
                                        Scalar::Union { .. } => {
                                            unreachable!("Enum tag should not be a union.")
                                        }
                                    };
                                    // For enums, tag is the only field and should have offset of 0.
                                    assert!(offsets.len() == 1 && offsets[0].bytes() == 0);
                                    let tag_data_bytes =
                                        vec![DataBytes::data(current_offset, tag_size)];

                                    // Retrieve data bytes for the fields.
                                    let mut fields_data_bytes = vec![];
                                    // Iterate over all variants for the enum.
                                    for (index, variant) in variants.iter().enumerate() {
                                        let mut field_data_bytes_for_variant = vec![];
                                        let fields = ty_variants[index].fields();
                                        for field_idx in variant.fields_by_offset_order() {
                                            let field_offset = variant.offsets[field_idx].bytes();
                                            let field_ty = fields[field_idx].ty_with_args(args);
                                            field_data_bytes_for_variant.append(
                                                &mut data_bytes_for_ty(
                                                    machine_info,
                                                    field_ty,
                                                    field_offset + current_offset,
                                                )?,
                                            );
                                        }
                                        fields_data_bytes.push(field_data_bytes_for_variant);
                                    }

                                    if fields_data_bytes.is_empty() {
                                        // If there are no fields, return the tag data bytes.
                                        Ok(tag_data_bytes)
                                    } else if fields_data_bytes.iter().all(
                                        |data_bytes_for_variant| {
                                            // Byte layout for variant N.
                                            let byte_mask_for_variant = generate_byte_mask(
                                                layout.size.bytes(),
                                                data_bytes_for_variant.clone(),
                                            );
                                            // Byte layout for variant 0.
                                            let byte_mask_for_first = generate_byte_mask(
                                                layout.size.bytes(),
                                                fields_data_bytes.first().unwrap().clone(),
                                            );
                                            byte_mask_for_variant == byte_mask_for_first
                                        },
                                    ) {
                                        // If all fields have the same layout, return fields data
                                        // bytes.
                                        let mut total_data_bytes = tag_data_bytes;
                                        let mut field_data_bytes =
                                            fields_data_bytes.first().unwrap().clone();
                                        total_data_bytes.append(&mut field_data_bytes);
                                        Ok(total_data_bytes)
                                    } else {
                                        // Struct has multiple padding variants, Kani cannot
                                        // differentiate between them.
                                        Err(
                                            LayoutComputationError::EnumWithMultiplePaddingVariants(
                                                ty,
                                            ),
                                        )
                                    }
                                }
                            }
                        }
                        AdtKind::Union => unreachable!(),
                        AdtKind::Struct => {
                            let mut struct_data_bytes = vec![];
                            let fields = def.variants_iter().next().unwrap().fields();
                            for idx in layout.fields.fields_by_offset_order() {
                                let field_offset = offsets[idx].bytes();
                                let field_ty = fields[idx].ty_with_args(args);
                                struct_data_bytes.append(&mut data_bytes_for_ty(
                                    machine_info,
                                    field_ty,
                                    field_offset + current_offset,
                                )?);
                            }
                            Ok(struct_data_bytes)
                        }
                    }
                }
                RigidTy::Pat(base_ty, ..) => {
                    // This is similar to a structure with one field and with niche defined.
                    let mut pat_data_bytes = vec![];
                    pat_data_bytes.append(&mut data_bytes_for_ty(machine_info, *base_ty, 0)?);
                    Ok(pat_data_bytes)
                }
                RigidTy::Tuple(tys) => {
                    let mut tuple_data_bytes = vec![];
                    for idx in layout.fields.fields_by_offset_order() {
                        let field_offset = offsets[idx].bytes();
                        let field_ty = tys[idx];
                        tuple_data_bytes.append(&mut data_bytes_for_ty(
                            machine_info,
                            field_ty,
                            field_offset + current_offset,
                        )?);
                    }
                    Ok(tuple_data_bytes)
                }
                RigidTy::Bool
                | RigidTy::Char
                | RigidTy::Int(_)
                | RigidTy::Uint(_)
                | RigidTy::Float(_)
                | RigidTy::Never => {
                    unreachable!("Expected primitive layout for {ty:?}")
                }
                RigidTy::Str | RigidTy::Slice(_) | RigidTy::Array(_, _) => {
                    unreachable!("Expected array layout for {ty:?}")
                }
                RigidTy::RawPtr(_, _) | RigidTy::Ref(_, _, _) => Ok(match layout.value_repr {
                    ValueRepr::Scalar(Scalar::Initialized { value, .. }) => {
                        // Thin pointer, ABI is a single scalar.
                        vec![DataBytes::data(current_offset, value.size(machine_info))]
                    }
                    ValueRepr::ScalarPair {
                        a: Scalar::Initialized { value: value_first, .. },
                        b: Scalar::Initialized { value: value_second, .. },
                        ..
                    } => {
                        // Fat pointer, ABI is a scalar pair.
                        let FieldsShape::Arbitrary { offsets } = layout.fields else {
                            unreachable!()
                        };
                        // Since this is a scalar pair, only 2 elements are in the offsets vec.
                        assert!(offsets.len() == 2);
                        vec![
                            DataBytes::data(
                                current_offset + offsets[0].bytes(),
                                value_first.size(machine_info),
                            ),
                            DataBytes::data(
                                current_offset + offsets[1].bytes(),
                                value_second.size(machine_info),
                            ),
                        ]
                    }
                    _ => unreachable!("RigidTy::RawPtr | RigidTy::Ref with a non-scalar ABI."),
                }),
                // A closure or function item that captures nothing is zero-sized, so it has no
                // bytes whose initialization needs tracking. As of nightly-2026-11-11, calling a
                // closure through a function pointer argument can take a pointer to it.
                RigidTy::FnDef(_, _) | RigidTy::Closure(_, _) if layout.size.bytes() == 0 => {
                    Ok(vec![])
                }
                // A closure is laid out like the tuple of its captured variables, the last of its
                // generic arguments.
                RigidTy::Closure(_, args) => {
                    let Some(GenericArgKind::Type(upvars)) = args.0.last() else {
                        return Err(LayoutComputationError::UnsupportedType(ty));
                    };
                    let TyKind::RigidTy(RigidTy::Tuple(upvar_tys)) = upvars.kind() else {
                        return Err(LayoutComputationError::UnsupportedType(ty));
                    };
                    if upvar_tys.len() != offsets.len() {
                        return Err(LayoutComputationError::UnsupportedType(ty));
                    }
                    let mut closure_data_bytes = vec![];
                    for idx in layout.fields.fields_by_offset_order() {
                        closure_data_bytes.append(&mut data_bytes_for_ty(
                            machine_info,
                            upvar_tys[idx],
                            offsets[idx].bytes() + current_offset,
                        )?);
                    }
                    Ok(closure_data_bytes)
                }
                RigidTy::FnDef(_, _)
                | RigidTy::FnPtr(_)
                | RigidTy::Coroutine(_, _)
                | RigidTy::CoroutineClosure(_, _)
                | RigidTy::CoroutineWitness(_, _)
                | RigidTy::Foreign(_)
                | RigidTy::Dynamic(_, _) => Err(LayoutComputationError::UnsupportedType(ty)),
            }
        }
        // A union nested in the type: its bytes are tracked as union bytes (see [UnionMask]).
        FieldsShape::Union(_) => {
            Ok(vec![DataBytes { offset: current_offset, size: layout.size, kind: ByteKind::Union }])
        }
        FieldsShape::Array { .. } => Ok(vec![]),
    }
}

/// What a transmute from `from_ty` to `to_ty` requires of the initialization of its source.
pub enum TransmuteRequirement {
    /// The layouts are compatible: every data byte of `to_ty` is a data byte of `from_ty`.
    Compatible,
    /// Some data bytes of `to_ty` are bytes of unions in `from_ty` (e.g. `MaybeUninit::assume_init`,
    /// a transmute from `MaybeUninit<T>` to `T`): the source value's bytes marked in this mask must
    /// be initialized, which only its memory can tell. All other data bytes of `to_ty` are data
    /// bytes of `from_ty`.
    Dynamic(Vec<bool>),
    /// A data byte of `to_ty` is padding in `from_ty`, or the layouts cannot be compared.
    Incompatible,
}

/// The initialization the transmute from `from_ty` to `to_ty` requires of its source.
pub fn transmute_requirement(from_ty: &Ty, to_ty: &Ty) -> TransmuteRequirement {
    let (Ok(from), Ok(to)) = (PointeeInfo::from_ty(*from_ty), PointeeInfo::from_ty(*to_ty)) else {
        return TransmuteRequirement::Incompatible;
    };
    // A union's bytes are all union bytes; it has no data bytes of its own.
    let (from_data, from_union) = match from.layout() {
        PointeeLayout::Sized { layout, union_mask } => (layout.clone(), union_mask.clone()),
        PointeeLayout::Union { .. } => {
            let size = from.layout().maybe_size().unwrap();
            (vec![false; size], vec![true; size])
        }
        _ => return TransmuteRequirement::Incompatible,
    };
    let to_data = match to.layout() {
        PointeeLayout::Sized { layout, .. } => layout.clone(),
        // The target's union bytes need not be initialized.
        PointeeLayout::Union { .. } => vec![false; to.layout().maybe_size().unwrap()],
        _ => return TransmuteRequirement::Incompatible,
    };
    if to_data.len() > from_data.len() {
        return TransmuteRequirement::Incompatible;
    }
    let is_union = |i: usize| from_union.get(i).copied().unwrap_or(false);
    let mut dynamic = vec![false; from_data.len()];
    for (i, needed) in to_data.iter().enumerate() {
        if *needed && !from_data[i] {
            if !is_union(i) {
                return TransmuteRequirement::Incompatible;
            }
            dynamic[i] = true;
        }
    }
    if dynamic.iter().any(|byte| *byte) {
        TransmuteRequirement::Dynamic(dynamic)
    } else {
        TransmuteRequirement::Compatible
    }
}

/// Returns true if `to_ty` has a smaller or equal size and padding bytes in `from_ty` are padding
/// bytes in `to_ty`.
pub fn tys_layout_compatible_to_size(from_ty: &Ty, to_ty: &Ty) -> bool {
    tys_layout_cmp_to_size(from_ty, to_ty, |from_byte, to_byte| from_byte || !to_byte)
}

/// Returns true if `to_ty` has a smaller or equal size and padding bytes in `from_ty` are padding
/// bytes in `to_ty`.
pub fn tys_layout_equal_to_size(from_ty: &Ty, to_ty: &Ty) -> bool {
    tys_layout_cmp_to_size(from_ty, to_ty, |from_byte, to_byte| from_byte == to_byte)
        && union_masks_equal_to_size(from_ty, to_ty)
}

/// Whether the bytes of nested unions (see [UnionMask]) are the same in both types, up to the
/// size of `to_ty`. A union byte and a padding byte are both not data bytes, but they differ: the
/// state of a union byte is what was last written to it.
fn union_masks_equal_to_size(from_ty: &Ty, to_ty: &Ty) -> bool {
    // The size of the type, and its union mask.
    let layout = |ty: &Ty| match PointeeInfo::from_ty(*ty).map(|info| info.layout) {
        Ok(PointeeLayout::Sized { layout, union_mask }) => Some((layout.len(), union_mask)),
        Ok(PointeeLayout::Slice { element_layout, element_union_mask }) => {
            Some((element_layout.len(), element_union_mask))
        }
        _ => None,
    };
    let (Some((_, from)), Some((to_size, to))) = (layout(from_ty), layout(to_ty)) else {
        return false;
    };
    // An empty mask means no union bytes.
    let is_union = |mask: &UnionMask, i: usize| mask.get(i).copied().unwrap_or(false);
    (0..to_size).all(|i| is_union(&from, i) == is_union(&to, i))
}

/// Returns true if `to_ty` has a smaller or equal size and comparator function returns true for all
/// byte initialization value pairs up to size.
fn tys_layout_cmp_to_size(from_ty: &Ty, to_ty: &Ty, cmp: impl Fn(bool, bool) -> bool) -> bool {
    // Retrieve layouts to assess compatibility.
    let from_ty_info = PointeeInfo::from_ty(*from_ty);
    let to_ty_info = PointeeInfo::from_ty(*to_ty);
    if let (Ok(from_ty_info), Ok(to_ty_info)) = (from_ty_info, to_ty_info) {
        let from_ty_layout = match from_ty_info.layout() {
            PointeeLayout::Sized { layout, .. } => layout,
            PointeeLayout::Slice { element_layout, .. } => element_layout,
            PointeeLayout::TraitObject | PointeeLayout::Union { .. } => return false,
        };
        let to_ty_layout = match to_ty_info.layout() {
            PointeeLayout::Sized { layout, .. } => layout,
            PointeeLayout::Slice { element_layout, .. } => element_layout,
            PointeeLayout::TraitObject | PointeeLayout::Union { .. } => return false,
        };
        // Ensure `to_ty_layout` does not have a larger size.
        if to_ty_layout.len() <= from_ty_layout.len() {
            // Check data and padding bytes pair-wise.
            if from_ty_layout.iter().zip(to_ty_layout.iter()).all(
                |(from_ty_layout_byte, to_ty_layout_byte)| {
                    // Run comparator on each pair.
                    cmp(*from_ty_layout_byte, *to_ty_layout_byte)
                },
            ) {
                return true;
            }
        }
    };
    false
}
