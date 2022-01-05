// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// This module introduces the Invariant trait as well as implementation for commonly used types.

/// Types that implement a check to ensure its value is valid and safe to be used. See
/// https://doc.rust-lang.org/stable/nomicon/what-unsafe-does.html for examples of valid values.
///
/// Implementations of Invariant traits must ensure that the current bit values of the given type
/// is valid and that all its invariants hold.
pub unsafe trait Invariant {
    fn is_valid(&self) -> bool;
}

unsafe impl Invariant for bool {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        let byte = u8::from(*self);
        byte == 0 || byte == 1
    }
}

unsafe impl Invariant for u8 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for u16 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for u32 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for u64 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for u128 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for usize {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for i8 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for i16 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for i32 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for i64 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for i128 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

/// We do not constraint floating points values per type spec. Users must add assumptions to their
/// verification code if they want to eliminate NaN, infinite, or subnormal.
unsafe impl Invariant for f32 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

/// We do not constraint floating points values per type spec. Users must add assumptions to their
/// verification code if they want to eliminate NaN, infinite, or subnormal.
unsafe impl Invariant for f64 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

unsafe impl Invariant for isize {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

/// Validate that a char is not outside the ranges [0x0, 0xD7FF] and [0xE000, 0x10FFFF]
/// Ref: https://doc.rust-lang.org/stable/nomicon/what-unsafe-does.html
unsafe impl Invariant for char {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        // RMC translates char into i32.
        let val = *self as i32;
        val <= 0xD7FF || (val >= 0xE000 && val <= 0x10FFFF)
    }
}

unsafe impl<T> Invariant for Option<T>
where
    T: Invariant,
{
    #[inline(always)]
    fn is_valid(&self) -> bool {
        if let Some(v) = self { v.is_valid() } else { matches!(*self, None) }
    }
}

unsafe impl<T, E> Invariant for Result<T, E>
where
    T: Invariant,
    E: Invariant,
{
    #[inline(always)]
    fn is_valid(&self) -> bool {
        if let Ok(v) = self {
            v.is_valid()
        } else if let Err(e) = self {
            e.is_valid()
        } else {
            false
        }
    }
}
