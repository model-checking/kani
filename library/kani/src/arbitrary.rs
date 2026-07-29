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

/// The models below are used by the compiler to generate nondeterministic
/// `Box<T>`/`Rc<T>`/`Arc<T>` values for automatic harnesses when `T` does not implement
/// `Arbitrary` in source code but the compiler can derive it: the `kani::any::<T>()` calls in
/// their bodies are then replaced with the compiler-synthesized implementation. (For `T`s that
/// do implement `Arbitrary`, the `Arbitrary` implementations above are resolved directly
/// instead.)
/// These models are *optional*: they require `alloc` and thus have no `core::kani`
/// counterpart, c.f. `KaniModel::is_optional`.
#[kanitool::fn_marker = "AnyBoxModel"]
#[inline(never)]
#[doc(hidden)]
pub fn any_box<T: Arbitrary>() -> Box<T> {
    Box::new(crate::any())
}

#[kanitool::fn_marker = "AnyRcModel"]
#[inline(never)]
#[doc(hidden)]
pub fn any_rc<T: Arbitrary>() -> std::rc::Rc<T> {
    std::rc::Rc::new(crate::any())
}

#[kanitool::fn_marker = "AnyArcModel"]
#[inline(never)]
#[doc(hidden)]
pub fn any_arc<T: Arbitrary>() -> std::sync::Arc<T> {
    std::sync::Arc::new(crate::any())
}

impl Arbitrary for std::time::Duration {
    fn any() -> Self {
        const NANOS_PER_SEC: u32 = 1_000_000_000;
        let nanos = u32::any();
        crate::assume(nanos < NANOS_PER_SEC);
        std::time::Duration::new(u64::any(), nanos)
    }
}
