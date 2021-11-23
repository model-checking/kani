// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// This module introduces the Invariant trait and as well as implementation for commonly used types.

/// Types that implement a check to ensure its value is valid and safe to be used. See
/// https://doc.rust-lang.org/stable/nomicon/what-unsafe-does.html for examples of valid values.
pub trait Invariant {
    fn is_valid(&self) -> bool;
}

impl Invariant for bool {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        let byte = u8::from(*self);
        byte == 0 || byte == 1
    }
}

impl Invariant for u8 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for u16 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for u32 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for u64 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for u128 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for usize {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for i8 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for i16 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for i32 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for i64 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for i128 {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

impl Invariant for isize {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        true
    }
}

/// Verifies that:
/// - a char is not outside the ranges [0x0, 0xD7FF] and [0xE000, 0x10FFFF]
/// Ref: https://doc.rust-lang.org/stable/nomicon/what-unsafe-does.html
impl Invariant for char {
    #[inline(always)]
    fn is_valid(&self) -> bool {
        let val = *self as u32;
        val <= 0xD7FF || (val >= 0xE000 && val <= 0x10FFFF)
    }
}
