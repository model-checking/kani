// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports the `core::str::pattern` searcher types backed
// by a `MultiCharEqSearcher`: `CharArraySearcher`, `CharArrayRefSearcher` and
// `CharSliceSearcher`, c.f. the `AnyCharArraySearcher`, `AnyCharArrayRefSearcher` and
// `AnyCharSliceSearcher` models. The generated harness produces a searcher over a
// nondeterministic haystack, bounded by the string bound (4, passed explicitly by the script),
// and a nondeterministic needle.
// Each type is taken by value, with the full set of cover checks on the generated searcher, and
// by mutable reference, with a single cover check: the reference only wraps the same by-value
// model, so repeating the full set there would add verification time without new information.
// `CharSliceSearcher` is also taken by shared reference (`haystack_len_is_not_three`).
// The "TEST NOTE" comments explain the expected result per function.

#![feature(pattern)]

use std::str::pattern::{
    CharArrayRefSearcher, CharArraySearcher, CharSliceSearcher, ReverseSearcher, SearchStep,
    Searcher,
};

/// Cover the states of the searcher `$s` refers to, without advancing it: every step is taken on
/// a clone. `$tag` names the function in the cover descriptions.
macro_rules! witness {
    ($s:expr, $tag:literal) => {{
        let searcher = $s;
        let haystack = searcher.haystack();
        kani::cover(haystack.is_empty(), concat!($tag, ": empty haystack"));
        kani::cover(!haystack.is_empty(), concat!($tag, ": non-empty haystack"));
        kani::cover(
            !haystack.is_empty() && !haystack.as_bytes()[0].is_ascii(),
            concat!($tag, ": haystack starting with a multi-byte char"),
        );

        // The first step from the front.
        let front = searcher.clone().next();
        kani::cover(matches!(front, SearchStep::Match(..)), concat!($tag, ": next is Match"));
        kani::cover(matches!(front, SearchStep::Reject(..)), concat!($tag, ": next is Reject"));
        kani::cover(matches!(front, SearchStep::Done), concat!($tag, ": next is Done"));
        // A searcher that has been advanced all the way has nothing left of a non-empty haystack.
        kani::cover(
            !haystack.is_empty() && matches!(front, SearchStep::Done),
            concat!($tag, ": exhausted non-empty haystack"),
        );
        // A searcher that has already been advanced from the front starts past index 0.
        let front_advanced = match front {
            SearchStep::Match(start, _) | SearchStep::Reject(start, _) => start > 0,
            SearchStep::Done => false,
        };
        kani::cover(front_advanced, concat!($tag, ": advanced from the front"));

        // The first step from the back.
        let back = searcher.clone().next_back();
        // A searcher that has already been advanced from the back ends before the haystack's end.
        let back_advanced = match back {
            SearchStep::Match(_, end) | SearchStep::Reject(_, end) => end < haystack.len(),
            SearchStep::Done => false,
        };
        kani::cover(back_advanced, concat!($tag, ": advanced from the back"));
        kani::cover(front_advanced && back_advanced, concat!($tag, ": advanced from both ends"));
    }};
}

/// Cover that the searcher `$s` refers to was generated, for the by-reference variants.
macro_rules! witness_ref {
    ($s:expr, $tag:literal) => {{
        kani::cover(!$s.haystack().is_empty(), concat!($tag, ": non-empty haystack"));
    }};
}

// TEST NOTE: should PASS; the cover checks show which searcher states are generated.
pub fn char_array_value(s: CharArraySearcher<'_, 2>) {
    witness!(&s, "char_array_value");
}

// TEST NOTE: should PASS; the generated searcher can be passed by mutable reference.
pub fn char_array_mut(s: &mut CharArraySearcher<'_, 2>) {
    witness_ref!(s, "char_array_mut");
}

// TEST NOTE: should PASS; the cover checks show which searcher states are generated.
pub fn char_array_ref_searcher_value(s: CharArrayRefSearcher<'_, '_, 2>) {
    witness!(&s, "char_array_ref_searcher_value");
}

// TEST NOTE: should PASS; the generated searcher can be passed by mutable reference.
pub fn char_array_ref_searcher_mut(s: &mut CharArrayRefSearcher<'_, '_, 2>) {
    witness_ref!(s, "char_array_ref_searcher_mut");
}

// TEST NOTE: should PASS; the cover checks show which searcher states are generated.
pub fn char_slice_value(s: CharSliceSearcher<'_, '_>) {
    witness!(&s, "char_slice_value");
}

// TEST NOTE: should PASS; the generated searcher can be passed by mutable reference.
pub fn char_slice_mut(s: &mut CharSliceSearcher<'_, '_>) {
    witness_ref!(s, "char_slice_mut");
}

// TEST NOTE: should FAIL: the haystack is not fixed, so it may be 3 bytes long.
pub fn haystack_len_is_not_three(s: &CharSliceSearcher<'_, '_>) {
    assert!(s.haystack().len() != 3);
}
