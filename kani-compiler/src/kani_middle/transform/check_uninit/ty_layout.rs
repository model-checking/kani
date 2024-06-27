// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Utility functions that help calculate type layout.

use stable_mir::abi::{FieldsShape, LayoutShape, Scalar, TagEncoding, ValueAbi, VariantsShape};
use stable_mir::target::{MachineInfo, MachineSize};
use stable_mir::ty::{AdtKind, IndexedVal, RigidTy, Ty, TyKind, UintTy};
use stable_mir::CrateDef;

/// Represents a chunk of data bytes in a data structure.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct DataBytes {
    /// Offset in bytes.
    offset: usize,
    /// Size of this data chunk.
    size: MachineSize,
}

/// Bytewise mask, representing which bytes of a type are data and which are padding. Here, `false`
/// represents padding bytes and `true` represents data bytes.
type Layout = Vec<bool>;

/// Create a byte-wise mask from known chunks of data bytes.
fn generate_byte_mask(size_in_bytes: usize, data_chunks: Vec<DataBytes>) -> Vec<bool> {
    let mut layout_mask = vec![false; size_in_bytes];
    for data_bytes in data_chunks.iter() {
        for layout_item in
            layout_mask.iter_mut().skip(data_bytes.offset).take(data_bytes.size.bytes())
        {
            *layout_item = true;
        }
    }
    layout_mask
}

// Depending on whether the type is statically or dynamically sized,
// the layout of the element or the layout of the actual type is returned.
pub enum PointeeLayout {
    /// Layout of sized objects.
    Sized { layout: Layout },
    /// Layout of slices, *const/mut str is included in this case and treated as *const/mut [u8].
    Slice { element_layout: Layout },
    /// Trait objects have an arbitrary layout.
    TraitObject,
}

pub struct PointeeInfo {
    pointee_ty: Ty,
    layout: PointeeLayout,
}

impl PointeeInfo {
    pub fn from_ty(ty: Ty) -> Result<Self, String> {
        match ty.kind() {
            TyKind::RigidTy(rigid_ty) => match rigid_ty {
                RigidTy::Str => {
                    let slicee_ty = Ty::unsigned_ty(UintTy::U8);
                    let size_in_bytes = slicee_ty.layout().unwrap().shape().size.bytes();
                    let data_chunks = data_bytes_for_ty(&MachineInfo::target(), slicee_ty, 0)?;
                    let layout = PointeeLayout::Slice {
                        element_layout: generate_byte_mask(size_in_bytes, data_chunks),
                    };
                    Ok(PointeeInfo { pointee_ty: ty, layout })
                }
                RigidTy::Slice(slicee_ty) => {
                    let size_in_bytes = slicee_ty.layout().unwrap().shape().size.bytes();
                    let data_chunks = data_bytes_for_ty(&MachineInfo::target(), slicee_ty, 0)?;
                    let layout = PointeeLayout::Slice {
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
                            layout: generate_byte_mask(size_in_bytes, data_chunks),
                        };
                        Ok(PointeeInfo { pointee_ty: ty, layout })
                    } else {
                        Err(format!("Cannot determine type layout for type `{ty}`"))
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

/// Get a size of an initialized scalar.
fn scalar_ty_size(
    machine_info: &MachineInfo,
    layout_shape: LayoutShape,
    current_offset: usize,
) -> DataBytes {
    match layout_shape.abi {
        ValueAbi::Scalar(Scalar::Initialized { value, .. }) => {
            DataBytes { offset: current_offset, size: value.size(machine_info) }
        }
        ValueAbi::ScalarPair(
            Scalar::Initialized { value: value_first, .. },
            Scalar::Initialized { value: value_second, .. },
        ) => DataBytes {
            offset: current_offset,
            size: MachineSize::from_bits(
                value_first.size(machine_info).bits() + value_second.size(machine_info).bits(),
            ),
        },
        ValueAbi::Scalar(_)
        | ValueAbi::ScalarPair(_, _)
        | ValueAbi::Uninhabited
        | ValueAbi::Vector { .. }
        | ValueAbi::Aggregate { .. } => unreachable!(),
    }
}

/// Retrieve a set of data bytes with offsets for a type.
fn data_bytes_for_ty(
    machine_info: &MachineInfo,
    ty: Ty,
    current_offset: usize,
) -> Result<Vec<DataBytes>, String> {
    let layout = ty.layout().unwrap().shape();

    match layout.fields {
        FieldsShape::Primitive => Ok(vec![scalar_ty_size(machine_info, layout, current_offset)]),
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
            match ty.kind().rigid().expect(&format!("unexpected type: {ty:?}")) {
                RigidTy::Adt(def, args) => {
                    match def.kind() {
                        AdtKind::Enum => {
                            // Support basic enumeration forms
                            let ty_variants = def.variants();
                            match layout.variants {
                                VariantsShape::Single { index } => {
                                    // Only one variant is reachable. This behaves like a struct.
                                    let fields = ty_variants[index.to_index()].fields();
                                    let mut fields_data_bytes = vec![];
                                    for idx in layout.fields.fields_by_offset_order() {
                                        let field_offset = offsets[idx].bytes();
                                        let field_ty = fields[idx].ty_with_args(&args);
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
                                } => {
                                    Err(format!("Unsupported Enum `{}` check", def.trimmed_name()))?
                                }
                                VariantsShape::Multiple { variants, tag, .. } => {
                                    // Retrieve data bytes for the tag.
                                    let tag_size = match tag {
                                        Scalar::Initialized { value, .. } => {
                                            value.size(&machine_info)
                                        }
                                        Scalar::Union { .. } => {
                                            unreachable!("Enum tag should not be a union.")
                                        }
                                    };
                                    // For enums, tag is the only field and should have offset of 0.
                                    assert!(offsets.len() == 1 && offsets[0].bytes() == 0);
                                    let tag_data_bytes =
                                        vec![DataBytes { offset: current_offset, size: tag_size }];

                                    // Retrieve data bytes for the fields.
                                    let mut fields_data_bytes = vec![];
                                    // Iterate over all variants for the enum.
                                    for (index, variant) in variants.iter().enumerate() {
                                        let mut field_data_bytes_for_variant = vec![];
                                        let fields = ty_variants[index].fields();
                                        // Get offsets of all fields in a variant.
                                        let FieldsShape::Arbitrary { offsets: field_offsets } =
                                            variant.fields.clone()
                                        else {
                                            unreachable!()
                                        };
                                        for field_idx in variant.fields.fields_by_offset_order() {
                                            let field_offset = field_offsets[field_idx].bytes();
                                            let field_ty = fields[field_idx].ty_with_args(&args);
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
                                        Err(format!(
                                            "Unsupported Enum `{}` check",
                                            def.trimmed_name()
                                        ))
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
                                let field_ty = fields[idx].ty_with_args(&args);
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
                RigidTy::RawPtr(_, _) | RigidTy::Ref(_, _, _) => {
                    Ok(vec![scalar_ty_size(machine_info, layout, current_offset)])
                }
                RigidTy::FnDef(_, _)
                | RigidTy::FnPtr(_)
                | RigidTy::Closure(_, _)
                | RigidTy::Coroutine(_, _, _)
                | RigidTy::CoroutineWitness(_, _)
                | RigidTy::Foreign(_)
                | RigidTy::Dynamic(_, _, _) => Err(format!("Unsupported {ty:?}")),
            }
        }
        FieldsShape::Union(_) => Err(format!("Unsupported {ty:?}")),
        FieldsShape::Array { .. } => Ok(vec![]),
    }
}
