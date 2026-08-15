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
