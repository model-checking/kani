// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Single source of truth about which intrinsics we support.

use stable_mir::{
    mir::{mono::Instance, Mutability},
    ty::{FloatTy, IntTy, RigidTy, TyKind, UintTy},
};

// Enumeration of all intrinsics we support right now, with the last option being a catch-all. This
// way, adding an intrinsic would highlight all places where they are used.
#[allow(unused)]
#[derive(Clone, Debug)]
pub enum Intrinsic {
    AddWithOverflow,
    ArithOffset,
    AssertInhabited,
    AssertMemUninitializedValid,
    AssertZeroValid,
    Assume,
    AtomicAnd(String),
    AtomicCxchg(String),
    AtomicCxchgWeak(String),
    AtomicFence(String),
    AtomicLoad(String),
    AtomicMax(String),
    AtomicMin(String),
    AtomicNand(String),
    AtomicOr(String),
    AtomicSingleThreadFence(String),
    AtomicStore(String),
    AtomicUmax(String),
    AtomicUmin(String),
    AtomicXadd(String),
    AtomicXchg(String),
    AtomicXor(String),
    AtomicXsub(String),
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
    SimdShuffle(String),
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
    Unimplemented { name: String, issue_link: String },
}

/// Assert that top-level types of a function signature match the given patterns.
macro_rules! assert_sig_matches {
    ($sig:expr, $($input_type:pat),* => $output_type:pat) => {
        let inputs = $sig.inputs();
        let output = $sig.output();
        #[allow(unused_mut)]
        let mut index = 0;
        $(
            #[allow(unused_assignments)]
            {
                assert!(matches!(inputs[index].kind(), TyKind::RigidTy($input_type)));
                index += 1;
            }
        )*
        assert!(inputs.len() == index);
        assert!(matches!(output.kind(), TyKind::RigidTy($output_type)));
    }
}

impl Intrinsic {
    /// Create an intrinsic enum from a given intrinsic instance, shallowly validating the argument types.
    pub fn from_instance(intrinsic_instance: &Instance) -> Self {
        let intrinsic_str = intrinsic_instance.intrinsic_name().unwrap();
        let sig = intrinsic_instance.ty().kind().fn_sig().unwrap().skip_binder();
        match intrinsic_str.as_str() {
            "add_with_overflow" => {
                assert_sig_matches!(sig, _, _ => RigidTy::Tuple(_));
                Self::AddWithOverflow
            }
            "arith_offset" => {
                assert_sig_matches!(sig,
                    RigidTy::RawPtr(_, Mutability::Not),
                    RigidTy::Int(IntTy::Isize)
                    => RigidTy::RawPtr(_, Mutability::Not));
                Self::ArithOffset
            }
            "assert_inhabited" => {
                assert_sig_matches!(sig, => RigidTy::Tuple(_));
                Self::AssertInhabited
            }
            "assert_mem_uninitialized_valid" => {
                assert_sig_matches!(sig, => RigidTy::Tuple(_));
                Self::AssertMemUninitializedValid
            }
            "assert_zero_valid" => {
                assert_sig_matches!(sig, => RigidTy::Tuple(_));
                Self::AssertZeroValid
            }
            "assume" => {
                assert_sig_matches!(sig, RigidTy::Bool => RigidTy::Tuple(_));
                Self::Assume
            }
            name if name.starts_with("atomic_and") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicAnd(name.strip_prefix("atomic_and_").unwrap().into())
            }
            name if name.starts_with("atomic_cxchgweak") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _, _ => RigidTy::Tuple(_));
                Self::AtomicCxchgWeak(name.strip_prefix("atomic_cxchgweak_").unwrap().into())
            }
            name if name.starts_with("atomic_cxchg") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _, _ => RigidTy::Tuple(_));
                Self::AtomicCxchg(name.strip_prefix("atomic_cxchg_").unwrap().into())
            }
            name if name.starts_with("atomic_fence") => {
                assert_sig_matches!(sig, => RigidTy::Tuple(_));
                Self::AtomicFence(name.strip_prefix("atomic_fence_").unwrap().into())
            }
            name if name.starts_with("atomic_load") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not) => _);
                Self::AtomicLoad(name.strip_prefix("atomic_load_").unwrap().into())
            }
            name if name.starts_with("atomic_max") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicMax(name.strip_prefix("atomic_max_").unwrap().into())
            }
            name if name.starts_with("atomic_min") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicMin(name.strip_prefix("atomic_min_").unwrap().into())
            }
            name if name.starts_with("atomic_nand") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicNand(name.strip_prefix("atomic_nand_").unwrap().into())
            }
            name if name.starts_with("atomic_or") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicOr(name.strip_prefix("atomic_or_").unwrap().into())
            }
            name if name.starts_with("atomic_singlethreadfence") => {
                assert_sig_matches!(sig, => RigidTy::Tuple(_));
                Self::AtomicSingleThreadFence(
                    name.strip_prefix("atomic_singlethreadfence_").unwrap().into(),
                )
            }
            name if name.starts_with("atomic_store") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => RigidTy::Tuple(_));
                Self::AtomicStore(name.strip_prefix("atomic_store_").unwrap().into())
            }
            name if name.starts_with("atomic_umax") => {
                assert_sig_matches!(sig,
                    RigidTy::RawPtr(_, Mutability::Mut),
                    _
                    => _);
                Self::AtomicUmax(name.strip_prefix("atomic_umax_").unwrap().into())
            }
            name if name.starts_with("atomic_umin") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicUmin(name.strip_prefix("atomic_umin_").unwrap().into())
            }
            name if name.starts_with("atomic_xadd") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicXadd(name.strip_prefix("atomic_xadd_").unwrap().into())
            }
            name if name.starts_with("atomic_xchg") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicXchg(name.strip_prefix("atomic_xchg_").unwrap().into())
            }
            name if name.starts_with("atomic_xor") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicXor(name.strip_prefix("atomic_xor_").unwrap().into())
            }
            name if name.starts_with("atomic_xsub") => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => _);
                Self::AtomicXsub(name.strip_prefix("atomic_xsub_").unwrap().into())
            }
            "bitreverse" => {
                assert_sig_matches!(sig, _ => _);
                Self::Bitreverse
            }
            "black_box" => {
                assert_sig_matches!(sig, _ => _);
                Self::BlackBox
            }
            "breakpoint" => {
                assert_sig_matches!(sig, => RigidTy::Tuple(_));
                Self::Breakpoint
            }
            "bswap" => {
                assert_sig_matches!(sig, _ => _);
                Self::Bswap
            }
            "caller_location" => {
                assert_sig_matches!(sig, => RigidTy::Ref(_, _, Mutability::Not));
                Self::Unimplemented {
                    name: intrinsic_str,
                    issue_link: "https://github.com/model-checking/kani/issues/374".into(),
                }
            }
            "catch_unwind" => {
                assert_sig_matches!(sig, RigidTy::FnPtr(_), RigidTy::RawPtr(_, Mutability::Mut), RigidTy::FnPtr(_) => RigidTy::Int(IntTy::I32));
                Self::Unimplemented {
                    name: intrinsic_str,
                    issue_link: "https://github.com/model-checking/kani/issues/267".into(),
                }
            }
            "ceilf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::CeilF32
            }
            "ceilf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::CeilF64
            }
            "compare_bytes" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not), RigidTy::RawPtr(_, Mutability::Not), RigidTy::Uint(UintTy::Usize) => RigidTy::Int(IntTy::I32));
                Self::CompareBytes
            }
            "copy" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not), RigidTy::RawPtr(_, Mutability::Mut), RigidTy::Uint(UintTy::Usize) => RigidTy::Tuple(_));
                Self::Copy
            }
            "copy_nonoverlapping" => unreachable!(
                "Expected `core::intrinsics::unreachable` to be handled by `StatementKind::CopyNonOverlapping`"
            ),
            "copysignf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32), RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::CopySignF32
            }
            "copysignf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64), RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::CopySignF64
            }
            "cosf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::CosF32
            }
            "cosf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::CosF64
            }
            "ctlz" => {
                assert_sig_matches!(sig, _ => RigidTy::Uint(UintTy::U32));
                Self::Ctlz
            }
            "ctlz_nonzero" => {
                assert_sig_matches!(sig, _ => RigidTy::Uint(UintTy::U32));
                Self::CtlzNonZero
            }
            "ctpop" => {
                assert_sig_matches!(sig, _ => RigidTy::Uint(UintTy::U32));
                Self::Ctpop
            }
            "cttz" => {
                assert_sig_matches!(sig, _ => RigidTy::Uint(UintTy::U32));
                Self::Cttz
            }
            "cttz_nonzero" => {
                assert_sig_matches!(sig, _ => RigidTy::Uint(UintTy::U32));
                Self::CttzNonZero
            }
            "discriminant_value" => {
                assert_sig_matches!(sig, RigidTy::Ref(_, _, Mutability::Not) => _);
                Self::DiscriminantValue
            }
            "exact_div" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::ExactDiv
            }
            "exp2f32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::Exp2F32
            }
            "exp2f64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::Exp2F64
            }
            "expf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::ExpF32
            }
            "expf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::ExpF64
            }
            "fabsf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::FabsF32
            }
            "fabsf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::FabsF64
            }
            "fadd_fast" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::FaddFast
            }
            "fdiv_fast" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::FdivFast
            }
            "floorf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::FloorF32
            }
            "floorf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::FloorF64
            }
            "fmaf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32), RigidTy::Float(FloatTy::F32), RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::FmafF32
            }
            "fmaf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64), RigidTy::Float(FloatTy::F64), RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::FmafF64
            }
            "fmul_fast" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::FmulFast
            }
            "forget" => {
                assert_sig_matches!(sig, _ => RigidTy::Tuple(_));
                Self::Forget
            }
            "fsub_fast" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::FsubFast
            }
            "is_val_statically_known" => {
                assert_sig_matches!(sig, _ => RigidTy::Bool);
                Self::IsValStaticallyKnown
            }
            "likely" => {
                assert_sig_matches!(sig, RigidTy::Bool => RigidTy::Bool);
                Self::Likely
            }
            "log10f32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::Log10F32
            }
            "log10f64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::Log10F64
            }
            "log2f32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::Log2F32
            }
            "log2f64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::Log2F64
            }
            "logf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::LogF32
            }
            "logf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::LogF64
            }
            "maxnumf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32), RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::MaxNumF32
            }
            "maxnumf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64), RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::MaxNumF64
            }
            "min_align_of" => {
                assert_sig_matches!(sig, => RigidTy::Uint(UintTy::Usize));
                Self::MinAlignOf
            }
            "min_align_of_val" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not) => RigidTy::Uint(UintTy::Usize));
                Self::MinAlignOfVal
            }
            "minnumf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32), RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::MinNumF32
            }
            "minnumf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64), RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::MinNumF64
            }
            "mul_with_overflow" => {
                assert_sig_matches!(sig, _, _ => RigidTy::Tuple(_));
                Self::MulWithOverflow
            }
            "nearbyintf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::NearbyIntF32
            }
            "nearbyintf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::NearbyIntF64
            }
            "needs_drop" => {
                assert_sig_matches!(sig, => RigidTy::Bool);
                Self::NeedsDrop
            }
            // As of https://github.com/rust-lang/rust/pull/110822 the `offset` intrinsic is lowered to `mir::BinOp::Offset`
            "offset" => unreachable!(
                "Expected `core::intrinsics::unreachable` to be handled by `BinOp::OffSet`"
            ),
            "powf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32), RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::PowF32
            }
            "powf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64), RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::PowF64
            }
            "powif32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32), RigidTy::Int(IntTy::I32) => RigidTy::Float(FloatTy::F32));
                Self::PowIF32
            }
            "powif64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64), RigidTy::Int(IntTy::I32) => RigidTy::Float(FloatTy::F64));
                Self::PowIF64
            }
            "pref_align_of" => {
                assert_sig_matches!(sig, => RigidTy::Uint(UintTy::Usize));
                Self::PrefAlignOf
            }
            "ptr_guaranteed_cmp" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not), RigidTy::RawPtr(_, Mutability::Not) => RigidTy::Uint(UintTy::U8));
                Self::PtrGuaranteedCmp
            }
            "ptr_offset_from" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not), RigidTy::RawPtr(_, Mutability::Not) => RigidTy::Int(IntTy::Isize));
                Self::PtrOffsetFrom
            }
            "ptr_offset_from_unsigned" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not), RigidTy::RawPtr(_, Mutability::Not) => RigidTy::Uint(UintTy::Usize));
                Self::PtrOffsetFromUnsigned
            }
            "raw_eq" => {
                assert_sig_matches!(sig, RigidTy::Ref(_, _, Mutability::Not), RigidTy::Ref(_, _, Mutability::Not) => RigidTy::Bool);
                Self::RawEq
            }
            "rintf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::RintF32
            }
            "rintf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::RintF64
            }
            "rotate_left" => {
                assert_sig_matches!(sig, _, RigidTy::Uint(UintTy::U32) => _);
                Self::RotateLeft
            }
            "rotate_right" => {
                assert_sig_matches!(sig, _, RigidTy::Uint(UintTy::U32) => _);
                Self::RotateRight
            }
            "roundf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::RoundF32
            }
            "roundf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::RoundF64
            }
            "saturating_add" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SaturatingAdd
            }
            "saturating_sub" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SaturatingSub
            }
            "sinf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::SinF32
            }
            "sinf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::SinF64
            }
            "simd_add" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdAdd
            }
            "simd_and" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdAnd
            }
            "simd_div" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdDiv
            }
            "simd_rem" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdRem
            }
            "simd_eq" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdEq
            }
            "simd_extract" => {
                assert_sig_matches!(sig, _, RigidTy::Uint(UintTy::U32) => _);
                Self::SimdExtract
            }
            "simd_ge" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdGe
            }
            "simd_gt" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdGt
            }
            "simd_insert" => {
                assert_sig_matches!(sig, _, RigidTy::Uint(UintTy::U32), _ => _);
                Self::SimdInsert
            }
            "simd_le" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdLe
            }
            "simd_lt" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdLt
            }
            "simd_mul" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdMul
            }
            "simd_ne" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdNe
            }
            "simd_or" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdOr
            }
            "simd_shl" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdShl
            }
            "simd_shr" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdShr
            }
            name if name.starts_with("simd_shuffle") => {
                assert_sig_matches!(sig, _, _, _ => _);
                Self::SimdShuffle(name.strip_prefix("simd_shuffle").unwrap().into())
            }
            "simd_sub" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdSub
            }
            "simd_xor" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::SimdXor
            }
            "size_of" => unreachable!(),
            "size_of_val" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not) => RigidTy::Uint(UintTy::Usize));
                Self::SizeOfVal
            }
            "sqrtf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::SqrtF32
            }
            "sqrtf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::SqrtF64
            }
            "sub_with_overflow" => {
                assert_sig_matches!(sig, _, _ => RigidTy::Tuple(_));
                Self::SubWithOverflow
            }
            "transmute" => {
                assert_sig_matches!(sig, _ => _);
                Self::Transmute
            }
            "truncf32" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F32) => RigidTy::Float(FloatTy::F32));
                Self::TruncF32
            }
            "truncf64" => {
                assert_sig_matches!(sig, RigidTy::Float(FloatTy::F64) => RigidTy::Float(FloatTy::F64));
                Self::TruncF64
            }
            "type_id" => {
                assert_sig_matches!(sig, => RigidTy::Uint(UintTy::U128));
                Self::TypeId
            }
            "type_name" => {
                assert_sig_matches!(sig, => RigidTy::Ref(_, _, Mutability::Not));
                Self::TypeName
            }
            "typed_swap" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), RigidTy::RawPtr(_, Mutability::Mut) => RigidTy::Tuple(_));
                Self::TypedSwap
            }
            "unaligned_volatile_load" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not) => _);
                Self::UnalignedVolatileLoad
            }
            "unchecked_add" | "unchecked_mul" | "unchecked_shl" | "unchecked_shr"
            | "unchecked_sub" => {
                unreachable!("Expected intrinsic `{intrinsic_str}` to be lowered before codegen")
            }
            "unchecked_div" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::UncheckedDiv
            }
            "unchecked_rem" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::UncheckedRem
            }
            "unlikely" => {
                assert_sig_matches!(sig, RigidTy::Bool => RigidTy::Bool);
                Self::Unlikely
            }
            "unreachable" => unreachable!(
                "Expected `std::intrinsics::unreachable` to be handled by `TerminatorKind::Unreachable`"
            ),
            "volatile_copy_memory" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), RigidTy::RawPtr(_, Mutability::Not), RigidTy::Uint(UintTy::Usize) => RigidTy::Tuple(_));
                Self::VolatileCopyMemory
            }
            "volatile_copy_nonoverlapping_memory" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), RigidTy::RawPtr(_, Mutability::Not), RigidTy::Uint(UintTy::Usize) => RigidTy::Tuple(_));
                Self::VolatileCopyNonOverlappingMemory
            }
            "volatile_load" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not) => _);
                Self::VolatileLoad
            }
            "volatile_store" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), _ => RigidTy::Tuple(_));
                Self::VolatileStore
            }
            "vtable_size" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not) => RigidTy::Uint(UintTy::Usize));
                Self::VtableSize
            }
            "vtable_align" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Not) => RigidTy::Uint(UintTy::Usize));
                Self::VtableAlign
            }
            "wrapping_add" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::WrappingAdd
            }
            "wrapping_mul" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::WrappingMul
            }
            "wrapping_sub" => {
                assert_sig_matches!(sig, _, _ => _);
                Self::WrappingSub
            }
            "write_bytes" => {
                assert_sig_matches!(sig, RigidTy::RawPtr(_, Mutability::Mut), RigidTy::Uint(UintTy::U8), RigidTy::Uint(UintTy::Usize) => RigidTy::Tuple(_));
                Self::WriteBytes
            }
            // Unimplemented
            _ => Self::Unimplemented {
                name: intrinsic_str,
                issue_link: "https://github.com/model-checking/kani/issues/new/choose".into(),
            },
        }
    }
}
