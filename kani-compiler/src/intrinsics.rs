// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Single source of truth about which intrinsics we support.

// Enumeration of all intrinsics we support right now, with the last option being a catch-all. This
// way, adding an intrinsic would highlight all places where they are used.
#[allow(unused)]
pub enum Intrinsic<'a> {
    AddWithOverflow,
    ArithOffset,
    AssertInhabited,
    AssertMemUninitializedValid,
    AssertZeroValid,
    Assume,
    AtomicAnd(&'a str),
    AtomicCxchg(&'a str),
    AtomicCxchgWeak(&'a str),
    AtomicFence(&'a str),
    AtomicLoad(&'a str),
    AtomicMax(&'a str),
    AtomicMin(&'a str),
    AtomicNand(&'a str),
    AtomicOr(&'a str),
    AtomicSingleThreadFence(&'a str),
    AtomicStore(&'a str),
    AtomicUmax(&'a str),
    AtomicUmin(&'a str),
    AtomicXadd(&'a str),
    AtomicXchg(&'a str),
    AtomicXor(&'a str),
    AtomicXsub(&'a str),
    Bitreverse,
    BlackBox,
    Breakpoint,
    Bswap,
    CeilF32,
    CeilF64,
    CompareBytes,
    Copy,
    CopySignF32,
    CopySignF64,
    CosF32,
    CosF64,
    Ctlz,
    CtlzNonZero,
    Ctpop,
    Cttz,
    CttzNonZero,
    DiscriminantValue,
    ExactDiv,
    Exp2F32,
    Exp2F64,
    ExpF32,
    ExpF64,
    FabsF32,
    FabsF64,
    FaddFast,
    FdivFast,
    FloorF32,
    FloorF64,
    FmafF32,
    FmafF64,
    FmulFast,
    Forget,
    FsubFast,
    IsValStaticallyKnown,
    Likely,
    Log10F32,
    Log10F64,
    Log2F32,
    Log2F64,
    LogF32,
    LogF64,
    MaxNumF32,
    MaxNumF64,
    MinAlignOf,
    MinAlignOfVal,
    MinNumF32,
    MinNumF64,
    MulWithOverflow,
    NearbyIntF32,
    NearbyIntF64,
    NeedsDrop,
    PowF32,
    PowF64,
    PowIF32,
    PowIF64,
    PrefAlignOf,
    PtrGuaranteedCmp,
    PtrOffsetFrom,
    PtrOffsetFromUnsigned,
    RawEq,
    RetagBoxToRaw,
    RintF32,
    RintF64,
    RotateLeft,
    RotateRight,
    RoundF32,
    RoundF64,
    SaturatingAdd,
    SaturatingSub,
    SinF32,
    SinF64,
    SimdAdd,
    SimdAnd,
    SimdDiv,
    SimdRem,
    SimdEq,
    SimdExtract,
    SimdGe,
    SimdGt,
    SimdInsert,
    SimdLe,
    SimdLt,
    SimdMul,
    SimdNe,
    SimdOr,
    SimdShl,
    SimdShr,
    SimdShuffle(&'a str),
    SimdSub,
    SimdXor,
    SizeOfVal,
    SqrtF32,
    SqrtF64,
    SubWithOverflow,
    Transmute,
    TruncF32,
    TruncF64,
    TypeId,
    TypeName,
    TypedSwap,
    UnalignedVolatileLoad,
    UncheckedDiv,
    UncheckedRem,
    Unlikely,
    VolatileCopyMemory,
    VolatileCopyNonOverlappingMemory,
    VolatileLoad,
    VolatileStore,
    VtableSize,
    VtableAlign,
    WrappingAdd,
    WrappingMul,
    WrappingSub,
    WriteBytes,
    Unimplemented { name: &'a str, issue_link: &'a str },
}

impl<'a> Intrinsic<'a> {
    pub fn from_str(intrinsic_str: &'a str) -> Self {
        match intrinsic_str {
            "add_with_overflow" => Self::AddWithOverflow,
            "arith_offset" => Self::ArithOffset,
            "assert_inhabited" => Self::AssertInhabited,
            "assert_mem_uninitialized_valid" => Self::AssertMemUninitializedValid,
            "assert_zero_valid" => Self::AssertZeroValid,
            "assume" => Self::Assume,
            name if name.starts_with("atomic_and") => {
                Self::AtomicAnd(name.strip_prefix("atomic_and_").unwrap())
            }
            name if name.starts_with("atomic_cxchgweak") => {
                Self::AtomicCxchgWeak(name.strip_prefix("atomic_cxchgweak_").unwrap())
            }
            name if name.starts_with("atomic_cxchg") => {
                Self::AtomicCxchg(name.strip_prefix("atomic_cxchg_").unwrap())
            }
            name if name.starts_with("atomic_fence") => {
                Self::AtomicFence(name.strip_prefix("atomic_fence_").unwrap())
            }
            name if name.starts_with("atomic_load") => {
                Self::AtomicLoad(name.strip_prefix("atomic_load_").unwrap())
            }
            name if name.starts_with("atomic_max") => {
                Self::AtomicMax(name.strip_prefix("atomic_max_").unwrap())
            }
            name if name.starts_with("atomic_min") => {
                Self::AtomicMin(name.strip_prefix("atomic_min_").unwrap())
            }
            name if name.starts_with("atomic_nand") => {
                Self::AtomicNand(name.strip_prefix("atomic_nand_").unwrap())
            }
            name if name.starts_with("atomic_or") => {
                Self::AtomicOr(name.strip_prefix("atomic_or_").unwrap())
            }
            name if name.starts_with("atomic_singlethreadfence") => Self::AtomicSingleThreadFence(
                name.strip_prefix("atomic_singlethreadfence_").unwrap(),
            ),
            name if name.starts_with("atomic_store") => {
                Self::AtomicStore(name.strip_prefix("atomic_store_").unwrap())
            }
            name if name.starts_with("atomic_umax") => {
                Self::AtomicUmax(name.strip_prefix("atomic_umax_").unwrap())
            }
            name if name.starts_with("atomic_umin") => {
                Self::AtomicUmin(name.strip_prefix("atomic_umin_").unwrap())
            }
            name if name.starts_with("atomic_xadd") => {
                Self::AtomicXadd(name.strip_prefix("atomic_xadd_").unwrap())
            }
            name if name.starts_with("atomic_xchg") => {
                Self::AtomicXchg(name.strip_prefix("atomic_xchg_").unwrap())
            }
            name if name.starts_with("atomic_xor") => {
                Self::AtomicXor(name.strip_prefix("atomic_xor_").unwrap())
            }
            name if name.starts_with("atomic_xsub") => {
                Self::AtomicXsub(name.strip_prefix("atomic_xsub_").unwrap())
            }
            "bitreverse" => Self::Bitreverse,
            "black_box" => Self::BlackBox,
            "breakpoint" => Self::Breakpoint,
            "bswap" => Self::Bswap,
            "caller_location" => Self::Unimplemented {
                name: intrinsic_str,
                issue_link: "https://github.com/model-checking/kani/issues/374",
            },
            "catch_unwind" => Self::Unimplemented {
                name: intrinsic_str,
                issue_link: "https://github.com/model-checking/kani/issues/267",
            },
            "ceilf32" => Self::CeilF32,
            "ceilf64" => Self::CeilF64,
            "compare_bytes" => Self::CompareBytes,
            "copy" => Self::Copy,
            "copy_nonoverlapping" => unreachable!(
                "Expected `core::intrinsics::unreachable` to be handled by `StatementKind::CopyNonOverlapping`"
            ),
            "copysignf32" => Self::CopySignF32,
            "copysignf64" => Self::CopySignF64,
            "cosf32" => Self::CosF32,
            "cosf64" => Self::CosF64,
            "ctlz" => Self::Ctlz,
            "ctlz_nonzero" => Self::CtlzNonZero,
            "ctpop" => Self::Ctpop,
            "cttz" => Self::Cttz,
            "cttz_nonzero" => Self::CttzNonZero,
            "discriminant_value" => Self::DiscriminantValue,
            "exact_div" => Self::ExactDiv,
            "exp2f32" => Self::Exp2F32,
            "exp2f64" => Self::Exp2F64,
            "expf32" => Self::ExpF32,
            "expf64" => Self::ExpF64,
            "fabsf32" => Self::FabsF32,
            "fabsf64" => Self::FabsF64,
            "fadd_fast" => Self::FaddFast,
            "fdiv_fast" => Self::FdivFast,
            "floorf32" => Self::FloorF32,
            "floorf64" => Self::FloorF64,
            "fmaf32" => Self::FmafF32,
            "fmaf64" => Self::FmafF64,
            "fmul_fast" => Self::FmulFast,
            "forget" => Self::Forget,
            "fsub_fast" => Self::FsubFast,
            "is_val_statically_known" => Self::IsValStaticallyKnown,
            "likely" => Self::Likely,
            "log10f32" => Self::Log10F32,
            "log10f64" => Self::Log10F64,
            "log2f32" => Self::Log2F32,
            "log2f64" => Self::Log2F64,
            "logf32" => Self::LogF32,
            "logf64" => Self::LogF64,
            "maxnumf32" => Self::MaxNumF32,
            "maxnumf64" => Self::MaxNumF64,
            "min_align_of" => Self::MinAlignOf,
            "min_align_of_val" => Self::MinAlignOfVal,
            "minnumf32" => Self::MinNumF32,
            "minnumf64" => Self::MinNumF64,
            "mul_with_overflow" => Self::MulWithOverflow,
            "nearbyintf32" => Self::NearbyIntF32,
            "nearbyintf64" => Self::NearbyIntF64,
            "needs_drop" => Self::NeedsDrop,
            // As of https://github.com/rust-lang/rust/pull/110822 the `offset` intrinsic is lowered to `mir::BinOp::Offset`
            "offset" => unreachable!(
                "Expected `core::intrinsics::unreachable` to be handled by `BinOp::OffSet`"
            ),
            "powf32" => Self::PowF32,
            "powf64" => Self::PowF64,
            "powif32" => Self::PowIF32,
            "powif64" => Self::PowIF64,
            "pref_align_of" => Self::PrefAlignOf,
            "ptr_guaranteed_cmp" => Self::PtrGuaranteedCmp,
            "ptr_offset_from" => Self::PtrOffsetFrom,
            "ptr_offset_from_unsigned" => Self::PtrOffsetFromUnsigned,
            "raw_eq" => Self::RawEq,
            "retag_box_to_raw" => Self::RetagBoxToRaw,
            "rintf32" => Self::RintF32,
            "rintf64" => Self::RintF64,
            "rotate_left" => Self::RotateLeft,
            "rotate_right" => Self::RotateRight,
            "roundf32" => Self::RoundF32,
            "roundf64" => Self::RoundF64,
            "saturating_add" => Self::SaturatingAdd,
            "saturating_sub" => Self::SaturatingSub,
            "sinf32" => Self::SinF32,
            "sinf64" => Self::SinF64,
            "simd_add" => Self::SimdAdd,
            "simd_and" => Self::SimdAnd,
            "simd_div" => Self::SimdDiv,
            "simd_rem" => Self::SimdRem,
            "simd_eq" => Self::SimdEq,
            "simd_extract" => Self::SimdExtract,
            "simd_ge" => Self::SimdGe,
            "simd_gt" => Self::SimdGt,
            "simd_insert" => Self::SimdInsert,
            "simd_le" => Self::SimdLe,
            "simd_lt" => Self::SimdLt,
            "simd_mul" => Self::SimdMul,
            "simd_ne" => Self::SimdNe,
            "simd_or" => Self::SimdOr,
            "simd_shl" => Self::SimdShl,
            "simd_shr" => Self::SimdShr,
            name if name.starts_with("simd_shuffle") => {
                Self::SimdShuffle(name.strip_prefix("simd_shuffle").unwrap())
            }
            "simd_sub" => Self::SimdSub,
            "simd_xor" => Self::SimdXor,
            "size_of" => unreachable!(),
            "size_of_val" => Self::SizeOfVal,
            "sqrtf32" => Self::SqrtF32,
            "sqrtf64" => Self::SqrtF64,
            "sub_with_overflow" => Self::SubWithOverflow,
            "transmute" => Self::Transmute,
            "truncf32" => Self::TruncF32,
            "truncf64" => Self::TruncF64,
            "type_id" => Self::TypeId,
            "type_name" => Self::TypeName,
            "typed_swap" => Self::TypedSwap,
            "unaligned_volatile_load" => Self::UnalignedVolatileLoad,
            "unchecked_add" | "unchecked_mul" | "unchecked_shl" | "unchecked_shr"
            | "unchecked_sub" => {
                unreachable!("Expected intrinsic `{intrinsic_str}` to be lowered before codegen")
            }
            "unchecked_div" => Self::UncheckedDiv,
            "unchecked_rem" => Self::UncheckedRem,
            "unlikely" => Self::Unlikely,
            "unreachable" => unreachable!(
                "Expected `std::intrinsics::unreachable` to be handled by `TerminatorKind::Unreachable`"
            ),
            "volatile_copy_memory" => Self::VolatileCopyMemory,
            "volatile_copy_nonoverlapping_memory" => Self::VolatileCopyNonOverlappingMemory,
            "volatile_load" => Self::VolatileLoad,
            "volatile_store" => Self::VolatileStore,
            "vtable_size" => Self::VtableSize,
            "vtable_align" => Self::VtableAlign,
            "wrapping_add" => Self::WrappingAdd,
            "wrapping_mul" => Self::WrappingMul,
            "wrapping_sub" => Self::WrappingSub,
            "write_bytes" => Self::WriteBytes,
            // Unimplemented
            _ => Self::Unimplemented {
                name: intrinsic_str,
                issue_link: "https://github.com/model-checking/kani/issues/new/choose",
            },
        }
    }
}
