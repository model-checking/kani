// compile-flags: -Cpanic=abort
#![feature(core_intrinsics)]
#![crate_type = "lib"]

// EMIT_MIR lower_intrinsics.wrapping.LowerIntrinsics.diff
pub fn wrapping<T: Copy>(a: T, b: T) {
    let _x = core::intrinsics::wrapping_add(a, b);
    let _y = core::intrinsics::wrapping_sub(a, b);
    let _z = core::intrinsics::wrapping_mul(a, b);
}

// EMIT_MIR lower_intrinsics.size_of.LowerIntrinsics.diff
pub fn size_of<T>() -> usize {
    core::intrinsics::size_of::<T>()
}

// EMIT_MIR lower_intrinsics.align_of.LowerIntrinsics.diff
pub fn align_of<T>() -> usize {
    core::intrinsics::min_align_of::<T>()
}

// EMIT_MIR lower_intrinsics.forget.LowerIntrinsics.diff
pub fn forget<T>(t: T) {
    core::intrinsics::forget(t)
}

// EMIT_MIR lower_intrinsics.unreachable.LowerIntrinsics.diff
pub fn unreachable() -> ! {
    unsafe { core::intrinsics::unreachable() };
}

// EMIT_MIR lower_intrinsics.f_unit.PreCodegen.before.mir
pub fn f_unit() {
    f_dispatch(());
}


// EMIT_MIR lower_intrinsics.f_u64.PreCodegen.before.mir
pub fn f_u64() {
    f_dispatch(0u64);
}

#[inline(always)]
pub fn f_dispatch<T>(t: T) {
    if std::mem::size_of::<T>() == 0 {
        f_zst(t);
    } else {
        f_non_zst(t);
    }
}

#[inline(never)]
pub fn f_zst<T>(_t: T) {
}

#[inline(never)]
pub fn f_non_zst<T>(_t: T) {}

// EMIT_MIR lower_intrinsics.non_const.LowerIntrinsics.diff
pub fn non_const<T>() -> usize {
    // Check that lowering works with non-const operand as a func.
    let size_of_t = core::intrinsics::size_of::<T>;
    size_of_t()
}

pub enum E {
    A,
    B,
    C,
}

// EMIT_MIR lower_intrinsics.discriminant.LowerIntrinsics.diff
pub fn discriminant<T>(t: T) {
    core::intrinsics::discriminant_value(&t);
    core::intrinsics::discriminant_value(&0);
    core::intrinsics::discriminant_value(&());
    core::intrinsics::discriminant_value(&E::B);
}
