// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Minimal reproducer / regression test for a CBMC symbolic-execution
//! constant-propagation limitation exposed by rust-lang/rust#146436
//! ("Slice iter cleanup", first shipped in nightly-2025-12-04).
//!
//! After #146436, `ChunksExact::next` decides `Some`/`None` via
//! `self.v.split_at_checked(chunk_size).and_then(..)` -- i.e. through the
//! niche-encoded discriminant of `Option<(&[T], &[T])>`, which is the payload
//! reference's null-ness. When the slice's data pointer has been turned into a
//! symbolic null-select `(cond ? &obj : NULL)` by an upstream *symbolic-index*
//! `split_at` (whose `None` case is niche-encoded as a null pointer), CBMC's
//! symex can no longer fold that discriminant to a constant. The
//! `chunks_exact` loop then unwinds to the `--unwind` bound instead of its true
//! (concrete) trip count, blowing up symex/SAT.
//!
//! All three harnesses verify successfully -- the property is unchanged. The
//! point of the reproducer is the encoding *cost*: `poisoned_chunks_exact`
//! produces a much larger SAT instance than `control_clean` for the same-sized
//! concrete chunking work. If the underlying CBMC issue is fixed (or the
//! toolchain reverts the niche interaction), `poisoned_chunks_exact` bounds at
//! its true trip count again and the cost collapses back to the control's.

// Clean pointer (no symbolic-index split): the chunking loop bounds at its
// true trip count.
#[kani::proof]
#[kani::unwind(9)]
fn control_clean() {
    let chunks: [u8; 4] = kani::any();
    let mut sum = 0u64;
    for c in chunks.chunks_exact(2) {
        sum = sum.wrapping_add(c[0] as u64);
    }
    kani::cover!(sum != 999);
}

// A symbolic-index `split_at` poisons the slice's data pointer into a
// `(cond ? base : NULL)` select; `chunks_exact` (which decides Some/None via
// `split_at_checked(..).and_then(..)`) then over-unwinds even though `chunks`
// has a concrete length of 4.
#[kani::proof]
#[kani::unwind(9)]
fn poisoned_chunks_exact() {
    let arr: [u8; 16] = kani::any();
    let idx: usize = kani::any();
    kani::assume(idx <= 16);
    let (a, _) = arr.split_at(idx);
    if a.len() >= 4 {
        let chunks = &a[..4]; // concrete length 4, but a poisoned data pointer
        let mut sum = 0u64;
        for c in chunks.chunks_exact(2) {
            sum = sum.wrapping_add(c[0] as u64);
        }
        kani::cover!(sum != 999);
    }
}

// Negative control: the same poisoned slice fed to a direct `split_at_checked`
// loop (no `and_then` closure updating a captured `&mut`) is NOT affected -- it
// bounds normally. This shows both ingredients (the poisoned pointer *and*
// `chunks_exact`'s `and_then` form) are required to trigger the blow-up.
#[kani::proof]
#[kani::unwind(9)]
fn poisoned_split_at_checked() {
    let arr: [u8; 16] = kani::any();
    let idx: usize = kani::any();
    kani::assume(idx <= 16);
    let (a, _) = arr.split_at(idx);
    if a.len() >= 4 {
        let mut v = &a[..4];
        let mut c = 0u64;
        while let Some((_h, t)) = v.split_at_checked(2) {
            v = t;
            c += 1;
        }
        kani::cover!(c != 999);
    }
}
