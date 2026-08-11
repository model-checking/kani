// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the `Arbitrary` trait as well as implementation for
//! primitive types and other std containers.

use crate::Arbitrary;

impl<T> Arbitrary for std::boxed::Box<T>
where
    T: Arbitrary,
{
    fn any() -> Self {
        Box::new(T::any())
    }
}

impl<T> Arbitrary for std::rc::Rc<T>
where
    T: Arbitrary,
{
    fn any() -> Self {
        std::rc::Rc::new(T::any())
    }
}

impl<T> Arbitrary for std::sync::Arc<T>
where
    T: Arbitrary,
{
    fn any() -> Self {
        std::sync::Arc::new(T::any())
    }
}

impl Arbitrary for std::time::Duration {
    fn any() -> Self {
        const NANOS_PER_SEC: u32 = 1_000_000_000;
        let nanos = u32::any();
        crate::assume(nanos < NANOS_PER_SEC);
        std::time::Duration::new(u64::any(), nanos)
    }
}

/// Generate a slice of *unbounded* nondeterministic length: a fresh allocation of
/// nondeterministic size whose contents are nondeterministic, with element validity
/// established by `slice_validity_assume` (a compiler hook that emits a quantified
/// assumption constraining each element's raw bits to the element type's layout niche;
/// a no-op for element types whose every bit pattern is valid, e.g. integers).
///
/// This model is used by the compiler to generate nondeterministic `&[T]` arguments for
/// automatic harnesses (`kani autoharness`) when the element type qualifies; verification
/// results hold for ALL slice lengths (functions that iterate over the slice surface any
/// insufficient loop bound as an unwinding-assertion failure rather than passing silently).
///
/// This model is *optional*: it requires `alloc` and thus has no `core::kani` counterpart,
/// c.f. `KaniModel::is_optional`.
#[kanitool::fn_marker = "AnySliceRefUnboundedModel"]
#[inline(never)]
#[doc(hidden)]
pub fn any_slice_ref_unbounded<T>() -> &'static [T] {
    let len: usize = crate::any();
    let elem = std::mem::size_of::<T>();
    if elem == 0 {
        // ZST slices: no storage needed, any length is fine.
        return unsafe { std::slice::from_raw_parts(std::ptr::NonNull::dangling().as_ptr(), len) };
    }
    crate::assume(len <= (isize::MAX as usize) / elem);
    let layout = std::alloc::Layout::array::<T>(len.max(1)).unwrap();
    let ptr = unsafe { std::alloc::alloc(layout) };
    crate::assume(!ptr.is_null());
    slice_validity_assume::<T>(ptr, len);
    unsafe { std::slice::from_raw_parts(ptr as *const T, len) }
}

/// Generate a mutable slice of *unbounded* nondeterministic length: as
/// `any_slice_ref_unbounded`, but returning `&mut [T]`. Each call produces a fresh (leaked)
/// allocation, so the returned slice is exclusive by construction; writes through it are
/// unconstrained by other generated values.
///
/// This model is *optional*: it requires `alloc`, c.f. `KaniModel::is_optional`.
#[kanitool::fn_marker = "AnySliceMutUnboundedModel"]
#[inline(never)]
#[doc(hidden)]
pub fn any_slice_mut_unbounded<T>() -> &'static mut [T] {
    let len: usize = crate::any();
    let elem = std::mem::size_of::<T>();
    if elem == 0 {
        return unsafe {
            std::slice::from_raw_parts_mut(std::ptr::NonNull::dangling().as_ptr(), len)
        };
    }
    crate::assume(len <= (isize::MAX as usize) / elem);
    let layout = std::alloc::Layout::array::<T>(len.max(1)).unwrap();
    let ptr = unsafe { std::alloc::alloc(layout) };
    crate::assume(!ptr.is_null());
    slice_validity_assume::<T>(ptr, len);
    unsafe { std::slice::from_raw_parts_mut(ptr as *mut T, len) }
}

/// Generate a `Vec` of *unbounded* nondeterministic length: a fresh allocation of
/// nondeterministic size whose contents are nondeterministic, with element validity
/// established by `slice_validity_assume` (c.f. `any_slice_ref_unbounded`), handed to
/// `Vec::from_raw_parts` with `capacity == len` (the allocation came from the global
/// allocator with exactly that layout, as `Vec`'s safety contract requires; `Vec` frees it
/// on drop).
///
/// This model is used by the compiler to generate nondeterministic `Vec<T>` arguments for
/// automatic harnesses (`kani autoharness`) when the element type qualifies; verification
/// results hold for ALL lengths. Optional: requires `alloc`.
#[kanitool::fn_marker = "AnyVecUnboundedModel"]
#[inline(never)]
#[doc(hidden)]
pub fn any_vec_unbounded<T>() -> Vec<T> {
    let len: usize = crate::any();
    let elem = std::mem::size_of::<T>();
    if elem == 0 {
        // For ZSTs, Vec never allocates and uses a dangling pointer; constructing from a
        // dangling pointer with any len is the documented pattern (and loop-free, which
        // matters: generation code must not itself be bounded by unwinding).
        return unsafe {
            Vec::from_raw_parts(std::ptr::NonNull::dangling().as_ptr(), len, usize::MAX)
        };
    }
    crate::assume(len <= (isize::MAX as usize) / elem);
    let layout = std::alloc::Layout::array::<T>(len.max(1)).unwrap();
    let ptr = unsafe { std::alloc::alloc(layout) };
    crate::assume(!ptr.is_null());
    slice_validity_assume::<T>(ptr, len);
    unsafe { Vec::from_raw_parts(ptr as *mut T, len, len.max(1)) }
}

/// Compiler hook (c.f. `KaniHook::SliceValidityAssume`): assume that every element of the
/// `len`-element `T`-array at `ptr` has raw bits within `T`'s layout niche. Lowered directly
/// to a quantified goto assumption; a no-op when `T` has no niche. The default body is
/// unreachable: calls are always intercepted during code generation.
#[kanitool::fn_marker = "SliceValidityAssumeHook"]
#[inline(never)]
#[doc(hidden)]
pub fn slice_validity_assume<T>(_ptr: *const u8, _len: usize) {
    #[cfg(not(kani))]
    unreachable!("kani::slice_validity_assume is a verification-only hook");
}
