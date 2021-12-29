//! Computations on places -- field projections, going from mir::Place, and writing
//! into a place.
//! All high-level functions to write to memory work on places as destinations.

use std::convert::TryFrom;
use std::hash::Hash;

use rustc_ast::Mutability;
use rustc_macros::HashStable;
use rustc_middle::mir;
use rustc_middle::ty::layout::{LayoutOf, PrimitiveExt, TyAndLayout};
use rustc_middle::ty::{self, Ty};
use rustc_target::abi::{Abi, Align, FieldsShape, TagEncoding};
use rustc_target::abi::{HasDataLayout, Size, VariantIdx, Variants};

use super::{
    alloc_range, mir_assign_valid_types, AllocId, AllocRef, AllocRefMut, CheckInAllocMsg,
    ConstAlloc, ImmTy, Immediate, InterpCx, InterpResult, LocalValue, Machine, MemoryKind, OpTy,
    Operand, Pointer, PointerArithmetic, Provenance, Scalar, ScalarMaybeUninit,
};

#[derive(Copy, Clone, Hash, PartialEq, Eq, HashStable, Debug)]
/// Information required for the sound usage of a `MemPlace`.
pub enum MemPlaceMeta<Tag: Provenance = AllocId> {
    /// The unsized payload (e.g. length for slices or vtable pointer for trait objects).
    Meta(Scalar<Tag>),
    /// `Sized` types or unsized `extern type`
    None,
    /// The address of this place may not be taken. This protects the `MemPlace` from coming from
    /// a ZST Operand without a backing allocation and being converted to an integer address. This
    /// should be impossible, because you can't take the address of an operand, but this is a second
    /// protection layer ensuring that we don't mess up.
    Poison,
}

#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(MemPlaceMeta, 24);

impl<Tag: Provenance> MemPlaceMeta<Tag> {
    pub fn unwrap_meta(self) -> Scalar<Tag> {
        match self {
            Self::Meta(s) => s,
            Self::None | Self::Poison => {
                bug!("expected wide pointer extra data (e.g. slice length or trait object vtable)")
            }
        }
    }
    fn has_meta(self) -> bool {
        match self {
            Self::Meta(_) => true,
            Self::None | Self::Poison => false,
        }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, HashStable, Debug)]
pub struct MemPlace<Tag: Provenance = AllocId> {
    /// The pointer can be a pure integer, with the `None` tag.
    pub ptr: Pointer<Option<Tag>>,
    pub align: Align,
    /// Metadata for unsized places. Interpretation is up to the type.
    /// Must not be present for sized types, but can be missing for unsized types
    /// (e.g., `extern type`).
    pub meta: MemPlaceMeta<Tag>,
}

#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(MemPlace, 48);

#[derive(Copy, Clone, Hash, PartialEq, Eq, HashStable, Debug)]
pub enum Place<Tag: Provenance = AllocId> {
    /// A place referring to a value allocated in the `Memory` system.
    Ptr(MemPlace<Tag>),

    /// To support alloc-free locals, we are able to write directly to a local.
    /// (Without that optimization, we'd just always be a `MemPlace`.)
    Local { frame: usize, local: mir::Local },
}

#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(Place, 56);

#[derive(Copy, Clone, Debug)]
pub struct PlaceTy<'tcx, Tag: Provenance = AllocId> {
    place: Place<Tag>, // Keep this private; it helps enforce invariants.
    pub layout: TyAndLayout<'tcx>,
}

#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(PlaceTy<'_>, 72);

impl<'tcx, Tag: Provenance> std::ops::Deref for PlaceTy<'tcx, Tag> {
    type Target = Place<Tag>;
    #[inline(always)]
    fn deref(&self) -> &Place<Tag> {
        &self.place
    }
}

/// A MemPlace with its layout. Constructing it is only possible in this module.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct MPlaceTy<'tcx, Tag: Provenance = AllocId> {
    mplace: MemPlace<Tag>,
    pub layout: TyAndLayout<'tcx>,
}

#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
rustc_data_structures::static_assert_size!(MPlaceTy<'_>, 64);

impl<'tcx, Tag: Provenance> std::ops::Deref for MPlaceTy<'tcx, Tag> {
    type Target = MemPlace<Tag>;
    #[inline(always)]
    fn deref(&self) -> &MemPlace<Tag> {
        &self.mplace
    }
}

impl<'tcx, Tag: Provenance> From<MPlaceTy<'tcx, Tag>> for PlaceTy<'tcx, Tag> {
    #[inline(always)]
    fn from(mplace: MPlaceTy<'tcx, Tag>) -> Self {
        PlaceTy { place: Place::Ptr(mplace.mplace), layout: mplace.layout }
    }
}

impl<Tag: Provenance> MemPlace<Tag> {
    #[inline(always)]
    pub fn from_ptr(ptr: Pointer<Option<Tag>>, align: Align) -> Self {
        MemPlace { ptr, align, meta: MemPlaceMeta::None }
    }

    /// Adjust the provenance of the main pointer (metadata is unaffected).
    pub fn map_provenance(self, f: impl FnOnce(Option<Tag>) -> Option<Tag>) -> Self {
        MemPlace { ptr: self.ptr.map_provenance(f), ..self }
    }

    /// Turn a mplace into a (thin or wide) pointer, as a reference, pointing to the same space.
    /// This is the inverse of `ref_to_mplace`.
    #[inline(always)]
    pub fn to_ref(self, cx: &impl HasDataLayout) -> Immediate<Tag> {
        match self.meta {
            MemPlaceMeta::None => Immediate::from(Scalar::from_maybe_pointer(self.ptr, cx)),
            MemPlaceMeta::Meta(meta) => {
                Immediate::ScalarPair(Scalar::from_maybe_pointer(self.ptr, cx).into(), meta.into())
            }
            MemPlaceMeta::Poison => bug!(
                "MPlaceTy::dangling may never be used to produce a \
                place that will have the address of its pointee taken"
            ),
        }
    }

    #[inline]
    pub fn offset<'tcx>(
        self,
        offset: Size,
        meta: MemPlaceMeta<Tag>,
        cx: &impl HasDataLayout,
    ) -> InterpResult<'tcx, Self> {
        Ok(MemPlace {
            ptr: self.ptr.offset(offset, cx)?,
            align: self.align.restrict_for_offset(offset),
            meta,
        })
    }
}

impl<'tcx, Tag: Provenance> MPlaceTy<'tcx, Tag> {
    /// Produces a MemPlace that works for ZST but nothing else
    #[inline]
    pub fn dangling(layout: TyAndLayout<'tcx>) -> Self {
        let align = layout.align.abi;
        let ptr = Pointer::new(None, Size::from_bytes(align.bytes())); // no provenance, absolute address
        // `Poison` this to make sure that the pointer value `ptr` is never observable by the program.
        MPlaceTy { mplace: MemPlace { ptr, align, meta: MemPlaceMeta::Poison }, layout }
    }

    #[inline]
    pub fn offset(
        &self,
        offset: Size,
        meta: MemPlaceMeta<Tag>,
        layout: TyAndLayout<'tcx>,
        cx: &impl HasDataLayout,
    ) -> InterpResult<'tcx, Self> {
        Ok(MPlaceTy { mplace: self.mplace.offset(offset, meta, cx)?, layout })
    }

    #[inline]
    pub fn from_aligned_ptr(ptr: Pointer<Option<Tag>>, layout: TyAndLayout<'tcx>) -> Self {
        MPlaceTy { mplace: MemPlace::from_ptr(ptr, layout.align.abi), layout }
    }

    #[inline]
    pub(super) fn len(&self, cx: &impl HasDataLayout) -> InterpResult<'tcx, u64> {
        if self.layout.is_unsized() {
            // We need to consult `meta` metadata
            match self.layout.ty.kind() {
                ty::Slice(..) | ty::Str => self.mplace.meta.unwrap_meta().to_machine_usize(cx),
                _ => bug!("len not supported on unsized type {:?}", self.layout.ty),
            }
        } else {
            // Go through the layout.  There are lots of types that support a length,
            // e.g., SIMD types. (But not all repr(simd) types even have FieldsShape::Array!)
            match self.layout.fields {
                FieldsShape::Array { count, .. } => Ok(count),
                _ => bug!("len not supported on sized type {:?}", self.layout.ty),
            }
        }
    }

    #[inline]
    pub(super) fn vtable(&self) -> Scalar<Tag> {
        match self.layout.ty.kind() {
            ty::Dynamic(..) => self.mplace.meta.unwrap_meta(),
            _ => bug!("vtable not supported on type {:?}", self.layout.ty),
        }
    }
}

// These are defined here because they produce a place.
impl<'tcx, Tag: Provenance> OpTy<'tcx, Tag> {
    #[inline(always)]
    /// Note: do not call `as_ref` on the resulting place. This function should only be used to
    /// read from the resulting mplace, not to get its address back.
    pub fn try_as_mplace(&self) -> Result<MPlaceTy<'tcx, Tag>, ImmTy<'tcx, Tag>> {
        match **self {
            Operand::Indirect(mplace) => Ok(MPlaceTy { mplace, layout: self.layout }),
            Operand::Immediate(_) if self.layout.is_zst() => Ok(MPlaceTy::dangling(self.layout)),
            Operand::Immediate(imm) => Err(ImmTy::from_immediate(imm, self.layout)),
        }
    }

    #[inline(always)]
    /// Note: do not call `as_ref` on the resulting place. This function should only be used to
    /// read from the resulting mplace, not to get its address back.
    pub fn assert_mem_place(&self) -> MPlaceTy<'tcx, Tag> {
        self.try_as_mplace().unwrap()
    }
}

impl<Tag: Provenance> Place<Tag> {
    #[inline]
    pub fn assert_mem_place(self) -> MemPlace<Tag> {
        match self {
            Place::Ptr(mplace) => mplace,
            _ => bug!("assert_mem_place: expected Place::Ptr, got {:?}", self),
        }
    }
}

impl<'tcx, Tag: Provenance> PlaceTy<'tcx, Tag> {
    #[inline]
    pub fn assert_mem_place(self) -> MPlaceTy<'tcx, Tag> {
        MPlaceTy { mplace: self.place.assert_mem_place(), layout: self.layout }
    }
}

// separating the pointer tag for `impl Trait`, see https://github.com/rust-lang/rust/issues/54385
impl<'mir, 'tcx: 'mir, Tag, M> InterpCx<'mir, 'tcx, M>
where
    // FIXME: Working around https://github.com/rust-lang/rust/issues/54385
    Tag: Provenance + Eq + Hash + 'static,
    M: Machine<'mir, 'tcx, PointerTag = Tag>,
{
    /// Take a value, which represents a (thin or wide) reference, and make it a place.
    /// Alignment is just based on the type.  This is the inverse of `MemPlace::to_ref()`.
    ///
    /// Only call this if you are sure the place is "valid" (aligned and inbounds), or do not
    /// want to ever use the place for memory access!
    /// Generally prefer `deref_operand`.
    pub fn ref_to_mplace(
        &self,
        val: &ImmTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        let pointee_type =
            val.layout.ty.builtin_deref(true).expect("`ref_to_mplace` called on non-ptr type").ty;
        let layout = self.layout_of(pointee_type)?;
        let (ptr, meta) = match **val {
            Immediate::Scalar(ptr) => (ptr, MemPlaceMeta::None),
            Immediate::ScalarPair(ptr, meta) => (ptr, MemPlaceMeta::Meta(meta.check_init()?)),
        };

        let mplace = MemPlace {
            ptr: self.scalar_to_ptr(ptr.check_init()?),
            // We could use the run-time alignment here. For now, we do not, because
            // the point of tracking the alignment here is to make sure that the *static*
            // alignment information emitted with the loads is correct. The run-time
            // alignment can only be more restrictive.
            align: layout.align.abi,
            meta,
        };
        Ok(MPlaceTy { mplace, layout })
    }

    /// Take an operand, representing a pointer, and dereference it to a place -- that
    /// will always be a MemPlace.  Lives in `place.rs` because it creates a place.
    pub fn deref_operand(
        &self,
        src: &OpTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        let val = self.read_immediate(src)?;
        trace!("deref to {} on {:?}", val.layout.ty, *val);
        let mplace = self.ref_to_mplace(&val)?;
        self.check_mplace_access(mplace, CheckInAllocMsg::DerefTest)?;
        Ok(mplace)
    }

    #[inline]
    pub(super) fn get_alloc(
        &self,
        place: &MPlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx, Option<AllocRef<'_, 'tcx, M::PointerTag, M::AllocExtra>>> {
        assert!(!place.layout.is_unsized());
        assert!(!place.meta.has_meta());
        let size = place.layout.size;
        self.memory.get(place.ptr, size, place.align)
    }

    #[inline]
    pub(super) fn get_alloc_mut(
        &mut self,
        place: &MPlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx, Option<AllocRefMut<'_, 'tcx, M::PointerTag, M::AllocExtra>>> {
        assert!(!place.layout.is_unsized());
        assert!(!place.meta.has_meta());
        let size = place.layout.size;
        self.memory.get_mut(place.ptr, size, place.align)
    }

    /// Check if this mplace is dereferenceable and sufficiently aligned.
    fn check_mplace_access(
        &self,
        mplace: MPlaceTy<'tcx, M::PointerTag>,
        msg: CheckInAllocMsg,
    ) -> InterpResult<'tcx> {
        let (size, align) = self
            .size_and_align_of_mplace(&mplace)?
            .unwrap_or((mplace.layout.size, mplace.layout.align.abi));
        assert!(mplace.mplace.align <= align, "dynamic alignment less strict than static one?");
        let align = M::enforce_alignment(&self.memory.extra).then_some(align);
        self.memory.check_ptr_access_align(mplace.ptr, size, align.unwrap_or(Align::ONE), msg)?;
        Ok(())
    }

    /// Offset a pointer to project to a field of a struct/union. Unlike `place_field`, this is
    /// always possible without allocating, so it can take `&self`. Also return the field's layout.
    /// This supports both struct and array fields.
    ///
    /// This also works for arrays, but then the `usize` index type is restricting.
    /// For indexing into arrays, use `mplace_index`.
    #[inline(always)]
    pub fn mplace_field(
        &self,
        base: &MPlaceTy<'tcx, M::PointerTag>,
        field: usize,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        let offset = base.layout.fields.offset(field);
        let field_layout = base.layout.field(self, field);

        // Offset may need adjustment for unsized fields.
        let (meta, offset) = if field_layout.is_unsized() {
            // Re-use parent metadata to determine dynamic field layout.
            // With custom DSTS, this *will* execute user-defined code, but the same
            // happens at run-time so that's okay.
            match self.size_and_align_of(&base.meta, &field_layout)? {
                Some((_, align)) => (base.meta, offset.align_to(align)),
                None => {
                    // For unsized types with an extern type tail we perform no adjustments.
                    // NOTE: keep this in sync with `PlaceRef::project_field` in the codegen backend.
                    assert!(matches!(base.meta, MemPlaceMeta::None));
                    (base.meta, offset)
                }
            }
        } else {
            // base.meta could be present; we might be accessing a sized field of an unsized
            // struct.
            (MemPlaceMeta::None, offset)
        };

        // We do not look at `base.layout.align` nor `field_layout.align`, unlike
        // codegen -- mostly to see if we can get away with that
        base.offset(offset, meta, field_layout, self)
    }

    /// Index into an array.
    #[inline(always)]
    pub fn mplace_index(
        &self,
        base: &MPlaceTy<'tcx, M::PointerTag>,
        index: u64,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        // Not using the layout method because we want to compute on u64
        match base.layout.fields {
            FieldsShape::Array { stride, .. } => {
                let len = base.len(self)?;
                if index >= len {
                    // This can only be reached in ConstProp and non-rustc-MIR.
                    throw_ub!(BoundsCheckFailed { len, index });
                }
                let offset = stride * index; // `Size` multiplication
                // All fields have the same layout.
                let field_layout = base.layout.field(self, 0);

                assert!(!field_layout.is_unsized());
                base.offset(offset, MemPlaceMeta::None, field_layout, self)
            }
            _ => span_bug!(
                self.cur_span(),
                "`mplace_index` called on non-array type {:?}",
                base.layout.ty
            ),
        }
    }

    // Iterates over all fields of an array. Much more efficient than doing the
    // same by repeatedly calling `mplace_array`.
    pub(super) fn mplace_array_fields<'a>(
        &self,
        base: &'a MPlaceTy<'tcx, Tag>,
    ) -> InterpResult<'tcx, impl Iterator<Item = InterpResult<'tcx, MPlaceTy<'tcx, Tag>>> + 'a>
    {
        let len = base.len(self)?; // also asserts that we have a type where this makes sense
        let stride = match base.layout.fields {
            FieldsShape::Array { stride, .. } => stride,
            _ => span_bug!(self.cur_span(), "mplace_array_fields: expected an array layout"),
        };
        let layout = base.layout.field(self, 0);
        let dl = &self.tcx.data_layout;
        // `Size` multiplication
        Ok((0..len).map(move |i| base.offset(stride * i, MemPlaceMeta::None, layout, dl)))
    }

    fn mplace_subslice(
        &self,
        base: &MPlaceTy<'tcx, M::PointerTag>,
        from: u64,
        to: u64,
        from_end: bool,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        let len = base.len(self)?; // also asserts that we have a type where this makes sense
        let actual_to = if from_end {
            if from.checked_add(to).map_or(true, |to| to > len) {
                // This can only be reached in ConstProp and non-rustc-MIR.
                throw_ub!(BoundsCheckFailed { len: len, index: from.saturating_add(to) });
            }
            len.checked_sub(to).unwrap()
        } else {
            to
        };

        // Not using layout method because that works with usize, and does not work with slices
        // (that have count 0 in their layout).
        let from_offset = match base.layout.fields {
            FieldsShape::Array { stride, .. } => stride * from, // `Size` multiplication is checked
            _ => {
                span_bug!(self.cur_span(), "unexpected layout of index access: {:#?}", base.layout)
            }
        };

        // Compute meta and new layout
        let inner_len = actual_to.checked_sub(from).unwrap();
        let (meta, ty) = match base.layout.ty.kind() {
            // It is not nice to match on the type, but that seems to be the only way to
            // implement this.
            ty::Array(inner, _) => (MemPlaceMeta::None, self.tcx.mk_array(inner, inner_len)),
            ty::Slice(..) => {
                let len = Scalar::from_machine_usize(inner_len, self);
                (MemPlaceMeta::Meta(len), base.layout.ty)
            }
            _ => {
                span_bug!(self.cur_span(), "cannot subslice non-array type: `{:?}`", base.layout.ty)
            }
        };
        let layout = self.layout_of(ty)?;
        base.offset(from_offset, meta, layout, self)
    }

    pub(crate) fn mplace_downcast(
        &self,
        base: &MPlaceTy<'tcx, M::PointerTag>,
        variant: VariantIdx,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        // Downcasts only change the layout
        assert!(!base.meta.has_meta());
        Ok(MPlaceTy { layout: base.layout.for_variant(self, variant), ..*base })
    }

    /// Project into an mplace
    pub(super) fn mplace_projection(
        &self,
        base: &MPlaceTy<'tcx, M::PointerTag>,
        proj_elem: mir::PlaceElem<'tcx>,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        use rustc_middle::mir::ProjectionElem::*;
        Ok(match proj_elem {
            Field(field, _) => self.mplace_field(base, field.index())?,
            Downcast(_, variant) => self.mplace_downcast(base, variant)?,
            Deref => self.deref_operand(&base.into())?,

            Index(local) => {
                let layout = self.layout_of(self.tcx.types.usize)?;
                let n = self.access_local(self.frame(), local, Some(layout))?;
                let n = self.read_scalar(&n)?;
                let n = n.to_machine_usize(self)?;
                self.mplace_index(base, n)?
            }

            ConstantIndex { offset, min_length, from_end } => {
                let n = base.len(self)?;
                if n < min_length {
                    // This can only be reached in ConstProp and non-rustc-MIR.
                    throw_ub!(BoundsCheckFailed { len: min_length, index: n });
                }

                let index = if from_end {
                    assert!(0 < offset && offset <= min_length);
                    n.checked_sub(offset).unwrap()
                } else {
                    assert!(offset < min_length);
                    offset
                };

                self.mplace_index(base, index)?
            }

            Subslice { from, to, from_end } => self.mplace_subslice(base, from, to, from_end)?,
        })
    }

    /// Converts a repr(simd) place into a place where `place_index` accesses the SIMD elements.
    /// Also returns the number of elements.
    pub fn mplace_to_simd(
        &self,
        base: &MPlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx, (MPlaceTy<'tcx, M::PointerTag>, u64)> {
        // Basically we just transmute this place into an array following simd_size_and_type.
        // (Transmuting is okay since this is an in-memory place. We also double-check the size
        // stays the same.)
        let (len, e_ty) = base.layout.ty.simd_size_and_type(*self.tcx);
        let array = self.tcx.mk_array(e_ty, len);
        let layout = self.layout_of(array)?;
        assert_eq!(layout.size, base.layout.size);
        Ok((MPlaceTy { layout, ..*base }, len))
    }

    /// Gets the place of a field inside the place, and also the field's type.
    /// Just a convenience function, but used quite a bit.
    /// This is the only projection that might have a side-effect: We cannot project
    /// into the field of a local `ScalarPair`, we have to first allocate it.
    pub fn place_field(
        &mut self,
        base: &PlaceTy<'tcx, M::PointerTag>,
        field: usize,
    ) -> InterpResult<'tcx, PlaceTy<'tcx, M::PointerTag>> {
        // FIXME: We could try to be smarter and avoid allocation for fields that span the
        // entire place.
        let mplace = self.force_allocation(base)?;
        Ok(self.mplace_field(&mplace, field)?.into())
    }

    pub fn place_index(
        &mut self,
        base: &PlaceTy<'tcx, M::PointerTag>,
        index: u64,
    ) -> InterpResult<'tcx, PlaceTy<'tcx, M::PointerTag>> {
        let mplace = self.force_allocation(base)?;
        Ok(self.mplace_index(&mplace, index)?.into())
    }

    pub fn place_downcast(
        &self,
        base: &PlaceTy<'tcx, M::PointerTag>,
        variant: VariantIdx,
    ) -> InterpResult<'tcx, PlaceTy<'tcx, M::PointerTag>> {
        // Downcast just changes the layout
        Ok(match base.place {
            Place::Ptr(mplace) => {
                self.mplace_downcast(&MPlaceTy { mplace, layout: base.layout }, variant)?.into()
            }
            Place::Local { .. } => {
                let layout = base.layout.for_variant(self, variant);
                PlaceTy { layout, ..*base }
            }
        })
    }

    /// Projects into a place.
    pub fn place_projection(
        &mut self,
        base: &PlaceTy<'tcx, M::PointerTag>,
        &proj_elem: &mir::ProjectionElem<mir::Local, Ty<'tcx>>,
    ) -> InterpResult<'tcx, PlaceTy<'tcx, M::PointerTag>> {
        use rustc_middle::mir::ProjectionElem::*;
        Ok(match proj_elem {
            Field(field, _) => self.place_field(base, field.index())?,
            Downcast(_, variant) => self.place_downcast(base, variant)?,
            Deref => self.deref_operand(&self.place_to_op(base)?)?.into(),
            // For the other variants, we have to force an allocation.
            // This matches `operand_projection`.
            Subslice { .. } | ConstantIndex { .. } | Index(_) => {
                let mplace = self.force_allocation(base)?;
                self.mplace_projection(&mplace, proj_elem)?.into()
            }
        })
    }

    /// Converts a repr(simd) place into a place where `place_index` accesses the SIMD elements.
    /// Also returns the number of elements.
    pub fn place_to_simd(
        &mut self,
        base: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx, (MPlaceTy<'tcx, M::PointerTag>, u64)> {
        let mplace = self.force_allocation(base)?;
        self.mplace_to_simd(&mplace)
    }

    /// Computes a place. You should only use this if you intend to write into this
    /// place; for reading, a more efficient alternative is `eval_place_for_read`.
    pub fn eval_place(
        &mut self,
        place: mir::Place<'tcx>,
    ) -> InterpResult<'tcx, PlaceTy<'tcx, M::PointerTag>> {
        let mut place_ty = PlaceTy {
            // This works even for dead/uninitialized locals; we check further when writing
            place: Place::Local { frame: self.frame_idx(), local: place.local },
            layout: self.layout_of_local(self.frame(), place.local, None)?,
        };

        for elem in place.projection.iter() {
            place_ty = self.place_projection(&place_ty, &elem)?
        }

        trace!("{:?}", self.dump_place(place_ty.place));
        // Sanity-check the type we ended up with.
        debug_assert!(mir_assign_valid_types(
            *self.tcx,
            self.param_env,
            self.layout_of(self.subst_from_current_frame_and_normalize_erasing_regions(
                place.ty(&self.frame().body.local_decls, *self.tcx).ty
            )?)?,
            place_ty.layout,
        ));
        Ok(place_ty)
    }

    /// Write an immediate to a place
    #[inline(always)]
    pub fn write_immediate(
        &mut self,
        src: Immediate<M::PointerTag>,
        dest: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx> {
        self.write_immediate_no_validate(src, dest)?;

        if M::enforce_validity(self) {
            // Data got changed, better make sure it matches the type!
            self.validate_operand(&self.place_to_op(dest)?)?;
        }

        Ok(())
    }

    /// Write a scalar to a place
    #[inline(always)]
    pub fn write_scalar(
        &mut self,
        val: impl Into<ScalarMaybeUninit<M::PointerTag>>,
        dest: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx> {
        self.write_immediate(Immediate::Scalar(val.into()), dest)
    }

    /// Write a pointer to a place
    #[inline(always)]
    pub fn write_pointer(
        &mut self,
        ptr: impl Into<Pointer<Option<M::PointerTag>>>,
        dest: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx> {
        self.write_scalar(Scalar::from_maybe_pointer(ptr.into(), self), dest)
    }

    /// Write an immediate to a place.
    /// If you use this you are responsible for validating that things got copied at the
    /// right type.
    fn write_immediate_no_validate(
        &mut self,
        src: Immediate<M::PointerTag>,
        dest: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx> {
        if cfg!(debug_assertions) {
            // This is a very common path, avoid some checks in release mode
            assert!(!dest.layout.is_unsized(), "Cannot write unsized data");
            match src {
                Immediate::Scalar(ScalarMaybeUninit::Scalar(Scalar::Ptr(..))) => assert_eq!(
                    self.pointer_size(),
                    dest.layout.size,
                    "Size mismatch when writing pointer"
                ),
                Immediate::Scalar(ScalarMaybeUninit::Scalar(Scalar::Int(int))) => {
                    assert_eq!(int.size(), dest.layout.size, "Size mismatch when writing bits")
                }
                Immediate::Scalar(ScalarMaybeUninit::Uninit) => {} // uninit can have any size
                Immediate::ScalarPair(_, _) => {
                    // FIXME: Can we check anything here?
                }
            }
        }
        trace!("write_immediate: {:?} <- {:?}: {}", *dest, src, dest.layout.ty);

        // See if we can avoid an allocation. This is the counterpart to `try_read_immediate`,
        // but not factored as a separate function.
        let mplace = match dest.place {
            Place::Local { frame, local } => {
                match M::access_local_mut(self, frame, local)? {
                    Ok(local) => {
                        // Local can be updated in-place.
                        *local = LocalValue::Live(Operand::Immediate(src));
                        return Ok(());
                    }
                    Err(mplace) => {
                        // The local is in memory, go on below.
                        mplace
                    }
                }
            }
            Place::Ptr(mplace) => mplace, // already referring to memory
        };
        let dest = MPlaceTy { mplace, layout: dest.layout };

        // This is already in memory, write there.
        self.write_immediate_to_mplace_no_validate(src, &dest)
    }

    /// Write an immediate to memory.
    /// If you use this you are responsible for validating that things got copied at the
    /// right type.
    fn write_immediate_to_mplace_no_validate(
        &mut self,
        value: Immediate<M::PointerTag>,
        dest: &MPlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx> {
        // Note that it is really important that the type here is the right one, and matches the
        // type things are read at. In case `src_val` is a `ScalarPair`, we don't do any magic here
        // to handle padding properly, which is only correct if we never look at this data with the
        // wrong type.

        // Invalid places are a thing: the return place of a diverging function
        let tcx = *self.tcx;
        let mut alloc = match self.get_alloc_mut(dest)? {
            Some(a) => a,
            None => return Ok(()), // zero-sized access
        };

        // FIXME: We should check that there are dest.layout.size many bytes available in
        // memory.  The code below is not sufficient, with enough padding it might not
        // cover all the bytes!
        match value {
            Immediate::Scalar(scalar) => {
                match dest.layout.abi {
                    Abi::Scalar(_) => {} // fine
                    _ => span_bug!(
                        self.cur_span(),
                        "write_immediate_to_mplace: invalid Scalar layout: {:#?}",
                        dest.layout
                    ),
                }
                alloc.write_scalar(alloc_range(Size::ZERO, dest.layout.size), scalar)
            }
            Immediate::ScalarPair(a_val, b_val) => {
                // We checked `ptr_align` above, so all fields will have the alignment they need.
                // We would anyway check against `ptr_align.restrict_for_offset(b_offset)`,
                // which `ptr.offset(b_offset)` cannot possibly fail to satisfy.
                let (a, b) = match dest.layout.abi {
                    Abi::ScalarPair(a, b) => (a.value, b.value),
                    _ => span_bug!(
                        self.cur_span(),
                        "write_immediate_to_mplace: invalid ScalarPair layout: {:#?}",
                        dest.layout
                    ),
                };
                let (a_size, b_size) = (a.size(&tcx), b.size(&tcx));
                let b_offset = a_size.align_to(b.align(&tcx).abi);

                // It is tempting to verify `b_offset` against `layout.fields.offset(1)`,
                // but that does not work: We could be a newtype around a pair, then the
                // fields do not match the `ScalarPair` components.

                alloc.write_scalar(alloc_range(Size::ZERO, a_size), a_val)?;
                alloc.write_scalar(alloc_range(b_offset, b_size), b_val)
            }
        }
    }

    /// Copies the data from an operand to a place. This does not support transmuting!
    /// Use `copy_op_transmute` if the layouts could disagree.
    #[inline(always)]
    pub fn copy_op(
        &mut self,
        src: &OpTy<'tcx, M::PointerTag>,
        dest: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx> {
        self.copy_op_no_validate(src, dest)?;

        if M::enforce_validity(self) {
            // Data got changed, better make sure it matches the type!
            self.validate_operand(&self.place_to_op(dest)?)?;
        }

        Ok(())
    }

    /// Copies the data from an operand to a place. This does not support transmuting!
    /// Use `copy_op_transmute` if the layouts could disagree.
    /// Also, if you use this you are responsible for validating that things get copied at the
    /// right type.
    fn copy_op_no_validate(
        &mut self,
        src: &OpTy<'tcx, M::PointerTag>,
        dest: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx> {
        // We do NOT compare the types for equality, because well-typed code can
        // actually "transmute" `&mut T` to `&T` in an assignment without a cast.
        if !mir_assign_valid_types(*self.tcx, self.param_env, src.layout, dest.layout) {
            span_bug!(
                self.cur_span(),
                "type mismatch when copying!\nsrc: {:?},\ndest: {:?}",
                src.layout.ty,
                dest.layout.ty,
            );
        }

        // Let us see if the layout is simple so we take a shortcut, avoid force_allocation.
        let src = match self.try_read_immediate(src)? {
            Ok(src_val) => {
                assert!(!src.layout.is_unsized(), "cannot have unsized immediates");
                // Yay, we got a value that we can write directly.
                // FIXME: Add a check to make sure that if `src` is indirect,
                // it does not overlap with `dest`.
                return self.write_immediate_no_validate(*src_val, dest);
            }
            Err(mplace) => mplace,
        };
        // Slow path, this does not fit into an immediate. Just memcpy.
        trace!("copy_op: {:?} <- {:?}: {}", *dest, src, dest.layout.ty);

        // This interprets `src.meta` with the `dest` local's layout, if an unsized local
        // is being initialized!
        let (dest, size) = self.force_allocation_maybe_sized(dest, src.meta)?;
        let size = size.unwrap_or_else(|| {
            assert!(
                !dest.layout.is_unsized(),
                "Cannot copy into already initialized unsized place"
            );
            dest.layout.size
        });
        assert_eq!(src.meta, dest.meta, "Can only copy between equally-sized instances");

        self.memory
            .copy(src.ptr, src.align, dest.ptr, dest.align, size, /*nonoverlapping*/ true)
    }

    /// Copies the data from an operand to a place. The layouts may disagree, but they must
    /// have the same size.
    pub fn copy_op_transmute(
        &mut self,
        src: &OpTy<'tcx, M::PointerTag>,
        dest: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx> {
        if mir_assign_valid_types(*self.tcx, self.param_env, src.layout, dest.layout) {
            // Fast path: Just use normal `copy_op`
            return self.copy_op(src, dest);
        }
        // We still require the sizes to match.
        if src.layout.size != dest.layout.size {
            // FIXME: This should be an assert instead of an error, but if we transmute within an
            // array length computation, `typeck` may not have yet been run and errored out. In fact
            // most likey we *are* running `typeck` right now. Investigate whether we can bail out
            // on `typeck_results().has_errors` at all const eval entry points.
            debug!("Size mismatch when transmuting!\nsrc: {:#?}\ndest: {:#?}", src, dest);
            self.tcx.sess.delay_span_bug(
                self.cur_span(),
                "size-changing transmute, should have been caught by transmute checking",
            );
            throw_inval!(TransmuteSizeDiff(src.layout.ty, dest.layout.ty));
        }
        // Unsized copies rely on interpreting `src.meta` with `dest.layout`, we want
        // to avoid that here.
        assert!(
            !src.layout.is_unsized() && !dest.layout.is_unsized(),
            "Cannot transmute unsized data"
        );

        // The hard case is `ScalarPair`.  `src` is already read from memory in this case,
        // using `src.layout` to figure out which bytes to use for the 1st and 2nd field.
        // We have to write them to `dest` at the offsets they were *read at*, which is
        // not necessarily the same as the offsets in `dest.layout`!
        // Hence we do the copy with the source layout on both sides.  We also make sure to write
        // into memory, because if `dest` is a local we would not even have a way to write
        // at the `src` offsets; the fact that we came from a different layout would
        // just be lost.
        let dest = self.force_allocation(dest)?;
        self.copy_op_no_validate(
            src,
            &PlaceTy::from(MPlaceTy { mplace: *dest, layout: src.layout }),
        )?;

        if M::enforce_validity(self) {
            // Data got changed, better make sure it matches the type!
            self.validate_operand(&dest.into())?;
        }

        Ok(())
    }

    /// Ensures that a place is in memory, and returns where it is.
    /// If the place currently refers to a local that doesn't yet have a matching allocation,
    /// create such an allocation.
    /// This is essentially `force_to_memplace`.
    ///
    /// This supports unsized types and returns the computed size to avoid some
    /// redundant computation when copying; use `force_allocation` for a simpler, sized-only
    /// version.
    pub fn force_allocation_maybe_sized(
        &mut self,
        place: &PlaceTy<'tcx, M::PointerTag>,
        meta: MemPlaceMeta<M::PointerTag>,
    ) -> InterpResult<'tcx, (MPlaceTy<'tcx, M::PointerTag>, Option<Size>)> {
        let (mplace, size) = match place.place {
            Place::Local { frame, local } => {
                match M::access_local_mut(self, frame, local)? {
                    Ok(&mut local_val) => {
                        // We need to make an allocation.

                        // We need the layout of the local.  We can NOT use the layout we got,
                        // that might e.g., be an inner field of a struct with `Scalar` layout,
                        // that has different alignment than the outer field.
                        let local_layout =
                            self.layout_of_local(&self.stack()[frame], local, None)?;
                        // We also need to support unsized types, and hence cannot use `allocate`.
                        let (size, align) = self
                            .size_and_align_of(&meta, &local_layout)?
                            .expect("Cannot allocate for non-dyn-sized type");
                        let ptr = self.memory.allocate(size, align, MemoryKind::Stack)?;
                        let mplace = MemPlace { ptr: ptr.into(), align, meta };
                        if let LocalValue::Live(Operand::Immediate(value)) = local_val {
                            // Preserve old value.
                            // We don't have to validate as we can assume the local
                            // was already valid for its type.
                            let mplace = MPlaceTy { mplace, layout: local_layout };
                            self.write_immediate_to_mplace_no_validate(value, &mplace)?;
                        }
                        // Now we can call `access_mut` again, asserting it goes well,
                        // and actually overwrite things.
                        *M::access_local_mut(self, frame, local).unwrap().unwrap() =
                            LocalValue::Live(Operand::Indirect(mplace));
                        (mplace, Some(size))
                    }
                    Err(mplace) => (mplace, None), // this already was an indirect local
                }
            }
            Place::Ptr(mplace) => (mplace, None),
        };
        // Return with the original layout, so that the caller can go on
        Ok((MPlaceTy { mplace, layout: place.layout }, size))
    }

    #[inline(always)]
    pub fn force_allocation(
        &mut self,
        place: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        Ok(self.force_allocation_maybe_sized(place, MemPlaceMeta::None)?.0)
    }

    pub fn allocate(
        &mut self,
        layout: TyAndLayout<'tcx>,
        kind: MemoryKind<M::MemoryKind>,
    ) -> InterpResult<'static, MPlaceTy<'tcx, M::PointerTag>> {
        let ptr = self.memory.allocate(layout.size, layout.align.abi, kind)?;
        Ok(MPlaceTy::from_aligned_ptr(ptr.into(), layout))
    }

    /// Returns a wide MPlace of type `&'static [mut] str` to a new 1-aligned allocation.
    pub fn allocate_str(
        &mut self,
        str: &str,
        kind: MemoryKind<M::MemoryKind>,
        mutbl: Mutability,
    ) -> MPlaceTy<'tcx, M::PointerTag> {
        let ptr = self.memory.allocate_bytes(str.as_bytes(), Align::ONE, kind, mutbl);
        let meta = Scalar::from_machine_usize(u64::try_from(str.len()).unwrap(), self);
        let mplace =
            MemPlace { ptr: ptr.into(), align: Align::ONE, meta: MemPlaceMeta::Meta(meta) };

        let ty = self.tcx.mk_ref(
            self.tcx.lifetimes.re_static,
            ty::TypeAndMut { ty: self.tcx.types.str_, mutbl },
        );
        let layout = self.layout_of(ty).unwrap();
        MPlaceTy { mplace, layout }
    }

    /// Writes the discriminant of the given variant.
    pub fn write_discriminant(
        &mut self,
        variant_index: VariantIdx,
        dest: &PlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx> {
        // This must be an enum or generator.
        match dest.layout.ty.kind() {
            ty::Adt(adt, _) => assert!(adt.is_enum()),
            ty::Generator(..) => {}
            _ => span_bug!(
                self.cur_span(),
                "write_discriminant called on non-variant-type (neither enum nor generator)"
            ),
        }
        // Layout computation excludes uninhabited variants from consideration
        // therefore there's no way to represent those variants in the given layout.
        // Essentially, uninhabited variants do not have a tag that corresponds to their
        // discriminant, so we cannot do anything here.
        // When evaluating we will always error before even getting here, but ConstProp 'executes'
        // dead code, so we cannot ICE here.
        if dest.layout.for_variant(self, variant_index).abi.is_uninhabited() {
            throw_ub!(UninhabitedEnumVariantWritten)
        }

        match dest.layout.variants {
            Variants::Single { index } => {
                assert_eq!(index, variant_index);
            }
            Variants::Multiple {
                tag_encoding: TagEncoding::Direct,
                tag: tag_layout,
                tag_field,
                ..
            } => {
                // No need to validate that the discriminant here because the
                // `TyAndLayout::for_variant()` call earlier already checks the variant is valid.

                let discr_val =
                    dest.layout.ty.discriminant_for_variant(*self.tcx, variant_index).unwrap().val;

                // raw discriminants for enums are isize or bigger during
                // their computation, but the in-memory tag is the smallest possible
                // representation
                let size = tag_layout.value.size(self);
                let tag_val = size.truncate(discr_val);

                let tag_dest = self.place_field(dest, tag_field)?;
                self.write_scalar(Scalar::from_uint(tag_val, size), &tag_dest)?;
            }
            Variants::Multiple {
                tag_encoding:
                    TagEncoding::Niche { dataful_variant, ref niche_variants, niche_start },
                tag: tag_layout,
                tag_field,
                ..
            } => {
                // No need to validate that the discriminant here because the
                // `TyAndLayout::for_variant()` call earlier already checks the variant is valid.

                if variant_index != dataful_variant {
                    let variants_start = niche_variants.start().as_u32();
                    let variant_index_relative = variant_index
                        .as_u32()
                        .checked_sub(variants_start)
                        .expect("overflow computing relative variant idx");
                    // We need to use machine arithmetic when taking into account `niche_start`:
                    // tag_val = variant_index_relative + niche_start_val
                    let tag_layout = self.layout_of(tag_layout.value.to_int_ty(*self.tcx))?;
                    let niche_start_val = ImmTy::from_uint(niche_start, tag_layout);
                    let variant_index_relative_val =
                        ImmTy::from_uint(variant_index_relative, tag_layout);
                    let tag_val = self.binary_op(
                        mir::BinOp::Add,
                        &variant_index_relative_val,
                        &niche_start_val,
                    )?;
                    // Write result.
                    let niche_dest = self.place_field(dest, tag_field)?;
                    self.write_immediate(*tag_val, &niche_dest)?;
                }
            }
        }

        Ok(())
    }

    pub fn raw_const_to_mplace(
        &self,
        raw: ConstAlloc<'tcx>,
    ) -> InterpResult<'tcx, MPlaceTy<'tcx, M::PointerTag>> {
        // This must be an allocation in `tcx`
        let _ = self.tcx.global_alloc(raw.alloc_id);
        let ptr = self.global_base_pointer(Pointer::from(raw.alloc_id))?;
        let layout = self.layout_of(raw.ty)?;
        Ok(MPlaceTy::from_aligned_ptr(ptr.into(), layout))
    }

    /// Turn a place with a `dyn Trait` type into a place with the actual dynamic type.
    /// Also return some more information so drop doesn't have to run the same code twice.
    pub(super) fn unpack_dyn_trait(
        &self,
        mplace: &MPlaceTy<'tcx, M::PointerTag>,
    ) -> InterpResult<'tcx, (ty::Instance<'tcx>, MPlaceTy<'tcx, M::PointerTag>)> {
        let vtable = self.scalar_to_ptr(mplace.vtable()); // also sanity checks the type
        let (instance, ty) = self.read_drop_type_from_vtable(vtable)?;
        let layout = self.layout_of(ty)?;

        // More sanity checks
        if cfg!(debug_assertions) {
            let (size, align) = self.read_size_and_align_from_vtable(vtable)?;
            assert_eq!(size, layout.size);
            // only ABI alignment is preserved
            assert_eq!(align, layout.align.abi);
        }

        let mplace = MPlaceTy { mplace: MemPlace { meta: MemPlaceMeta::None, ..**mplace }, layout };
        Ok((instance, mplace))
    }
}
