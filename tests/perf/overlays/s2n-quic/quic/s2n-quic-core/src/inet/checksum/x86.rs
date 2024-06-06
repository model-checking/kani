// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(kani, allow(dead_code))]

use super::{Accumulator, LargeWriteFn, State, LARGE_WRITE_LEN};

#[cfg(target_arch = "x86")]
pub use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
pub use core::arch::x86_64::*;

/// Returns the optimized function for the given platform
///
/// If possible, this uses runtime feature detection so this should be cached.
#[inline]
pub fn probe() -> Option<LargeWriteFn> {
    probe_avx().or_else(probe_sse)
}

macro_rules! probe {
    ($feature:tt, $fun:ident) => {{
        #[cfg(all(feature = "std", not(any(miri, kani))))]
        {
            if std::is_x86_feature_detected!($feature) {
                return Some($fun);
            }
        }

        // no way to reliably detect features in no_std
        None
    }};
}

#[inline]
fn probe_avx() -> Option<LargeWriteFn> {
    probe!("avx", write_sized_avx)
}

#[inline]
fn probe_sse() -> Option<LargeWriteFn> {
    probe!("sse4.1", write_sized_sse)
}

/// Enable sse4.1 optimizations for the implementation
#[target_feature(enable = "sse4.1")]
pub unsafe fn write_sized_sse<'a>(state: &mut State, bytes: &'a [u8]) -> &'a [u8] {
    write_sized(state, bytes)
}

/// Enable avx optimizations for the implementation
#[target_feature(enable = "avx")]
pub unsafe fn write_sized_avx<'a>(state: &mut State, bytes: &'a [u8]) -> &'a [u8] {
    write_sized(state, bytes)
}

#[inline(always)]
unsafe fn write_sized<'a>(state: &mut State, mut bytes: &'a [u8]) -> &'a [u8] {
    assume!(
        bytes.len() >= LARGE_WRITE_LEN,
        "large write function should only be called with large byte buffers"
    );

    // Maintain two different sums to increase instruction parallelism
    let mut sum_a = Sum::new();
    let mut sum_b = Sum::new();

    // Unroll the loop to read 256 bytes at a time (16 16-byte blocks)
    while bytes.len() >= 256 {
        let (chunks, remaining) = bytes.split_at(256);
        bytes = remaining;

        let ptr = chunks.as_ptr() as *const __m128i;

        sum_a += ptr;
        sum_a += ptr.add(1);
        sum_a += ptr.add(2);
        sum_a += ptr.add(3);
        sum_a += ptr.add(4);
        sum_a += ptr.add(5);
        sum_a += ptr.add(6);
        sum_a += ptr.add(7);

        sum_b += ptr.add(8);
        sum_b += ptr.add(9);
        sum_b += ptr.add(10);
        sum_b += ptr.add(11);
        sum_b += ptr.add(12);
        sum_b += ptr.add(13);
        sum_b += ptr.add(14);
        sum_b += ptr.add(15);
    }

    // Unroll the loop to read 64 bytes at a time (4 16-byte blocks)
    while bytes.len() >= 64 {
        let (chunks, remaining) = bytes.split_at(64);
        bytes = remaining;

        let ptr = chunks.as_ptr() as *const __m128i;

        sum_a += ptr;
        sum_a += ptr.add(1);
        sum_b += ptr.add(2);
        sum_b += ptr.add(3);
    }

    // Finish reading the full 16-byte blocks
    while bytes.len() >= 16 {
        let (chunks, remaining) = bytes.split_at(16);
        bytes = remaining;

        let ptr = chunks.as_ptr() as *const __m128i;
        sum_a += ptr;
    }

    // Add up all of the sums and merge them into a single u32
    let sum = sum_a + sum_b;

    sum.extract(state);

    bytes
}

#[derive(Clone)]
struct Sum {
    a: __m128i,
    b: __m128i,
}

impl Sum {
    #[inline(always)]
    unsafe fn new() -> Self {
        Self { a: _mm_setzero_si128(), b: _mm_setzero_si128() }
    }

    #[inline(always)]
    unsafe fn add(&mut self, rhs: __m128i) {
        // Reads pairs of bytes into a 32-bit value
        //
        // Since we have 16 bytes as input, we need two outputs since we're doubling the bit-width
        let mask_a =
            _mm_setr_epi8(0x0, 0x1, -1, -1, 0x2, 0x3, -1, -1, 0x4, 0x5, -1, -1, 0x6, 0x7, -1, -1);
        let mask_b =
            _mm_setr_epi8(0x8, 0x9, -1, -1, 0xa, 0xb, -1, -1, 0xc, 0xd, -1, -1, 0xe, 0xf, -1, -1);

        // Add the shuffled counts to our current state
        self.a = _mm_add_epi32(self.a, _mm_shuffle_epi8(rhs, mask_a));
        self.b = _mm_add_epi32(self.b, _mm_shuffle_epi8(rhs, mask_b));
    }

    #[inline(always)]
    unsafe fn extract(self, state: &mut State) {
        let total = _mm_add_epi32(self.a, self.b);
        let total = _mm_hadd_epi32(total, _mm_setzero_si128());
        let total = _mm_hadd_epi32(total, _mm_setzero_si128());

        *state += _mm_extract_epi32(total, 0) as Accumulator;
    }
}

impl core::ops::AddAssign<__m128i> for Sum {
    #[inline(always)]
    fn add_assign(&mut self, rhs: __m128i) {
        unsafe { self.add(rhs) }
    }
}

impl core::ops::AddAssign<*const __m128i> for Sum {
    #[inline(always)]
    fn add_assign(&mut self, rhs: *const __m128i) {
        unsafe { self.add(_mm_loadu_si128(rhs)) }
    }
}

impl core::ops::Add for Sum {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        unsafe {
            let a = _mm_add_epi32(self.a, self.b);
            let b = _mm_add_epi32(rhs.a, rhs.b);
            Self { a, b }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::write_sized_generic_u16, *};
    use bolero::check;
    use core::num::Wrapping;

    /// Compares the x86-optimized function to the generic function and ensures consistency
    #[test]
    fn differential() {
        if let Some(write_sized_opt) = probe() {
            check!().for_each(|bytes| {
                if bytes.len() < LARGE_WRITE_LEN {
                    return;
                }

                let generic = {
                    let mut state = Wrapping(0);
                    write_sized_generic_u16::<2>(&mut state, bytes);
                    state.0
                };

                let actual = {
                    let mut state = Wrapping(0);
                    let bytes = unsafe { write_sized_opt(&mut state, bytes) };
                    write_sized_generic_u16::<2>(&mut state, bytes);
                    state.0
                };

                assert_eq!(generic.to_ne_bytes(), actual.to_ne_bytes())
            });
        }
    }
}
