//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Strategies to generate numeric values (as opposed to integers used as bit
//! fields).
//!
//! All strategies in this module shrink by binary searching towards 0.

macro_rules! numeric_api {
    ($typ:ident, $epsilon:expr) => {
        /// Type of the `ANY` constant.
        #[derive(Clone, Copy, Debug)]
        pub struct Any(());
        /// Generates integers with completely arbitrary values, uniformly
        /// distributed over the whole range.
        pub const ANY: Any = Any(());

        impl Strategy for Any {
            type Value = BinarySearch;

            fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(BinarySearch::new(runner.rng().gen()))
            }
        }

        impl Strategy for Range<$typ> {
            type Value = BinarySearch;

            fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
                let range = rand::distributions::Range::new(
                    self.start, self.end);
                Ok(BinarySearch::new_clamped(
                    self.start, range.ind_sample(runner.rng()),
                    self.end-$epsilon))
            }
        }

        impl Strategy for RangeFrom<$typ> {
            type Value = BinarySearch;

            fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
                // TODO `rand` has no way to express the inclusive-end range we
                // need here.
                let range = rand::distributions::Range::new(
                    self.start, ::std::$typ::MAX);
                Ok(BinarySearch::new_clamped(
                    self.start, range.ind_sample(runner.rng()),
                    ::std::$typ::MAX))
            }
        }

        impl Strategy for RangeTo<$typ> {
            type Value = BinarySearch;

            fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
                let range = rand::distributions::Range::new(
                    ::std::$typ::MIN, self.end);
                Ok(BinarySearch::new_clamped(
                    ::std::$typ::MIN, range.ind_sample(runner.rng()),
                    self.end))
            }
        }
    }
}

macro_rules! signed_integer_bin_search {
    ($typ:ident) => {
        #[allow(missing_docs)]
        pub mod $typ {
            use std::ops::{Range, RangeFrom, RangeTo};

            use rand::{self, Rng};
            use rand::distributions::IndependentSample;

            use strategy::*;
            use test_runner::TestRunner;

            /// Shrinks an integer towards 0, using binary search to find
            /// boundary points.
            #[derive(Clone, Copy, Debug)]
            pub struct BinarySearch {
                lo: $typ,
                curr: $typ,
                hi: $typ,
            }
            impl BinarySearch {
                /// Creates a new binary searcher starting at the given value.
                pub fn new(start: $typ) -> Self {
                    BinarySearch {
                        lo: 0,
                        curr: start,
                        hi: start,
                    }
                }

                /// Creates a new binary searcher which will not produce values
                /// on the other side of `lo` or `hi` from `start`. `lo` is
                /// inclusive, `hi` is exclusive.
                fn new_clamped(lo: $typ, start: $typ, hi: $typ) -> Self {
                    use std::cmp::{min, max};

                    BinarySearch {
                        lo: if start < 0 { min(0, hi-1) } else { max(0, lo) },
                        hi: start,
                        curr: start,
                    }
                }

                fn reposition(&mut self) -> bool {
                    // Won't ever overflow since lo starts at 0 and advances
                    // towards hi.
                    let interval = self.hi - self.lo;
                    let new_mid = self.lo + interval/2;

                    if new_mid == self.curr {
                        false
                    } else {
                        self.curr = new_mid;
                        true
                    }
                }

                fn magnitude_greater(lhs: $typ, rhs: $typ) -> bool {
                    if 0 == lhs {
                        false
                    } else if lhs < 0 {
                        lhs < rhs
                    } else {
                        lhs > rhs
                    }
                }
            }
            impl ValueTree for BinarySearch {
                type Value = $typ;

                fn current(&self) -> $typ {
                    self.curr
                }

                fn simplify(&mut self) -> bool {
                    if !BinarySearch::magnitude_greater(self.hi, self.lo) {
                        return false;
                    }

                    self.hi = self.curr;
                    self.reposition()
                }

                fn complicate(&mut self) -> bool {
                    if !BinarySearch::magnitude_greater(self.hi, self.lo) {
                        return false;
                    }

                    self.lo = self.curr + if self.hi < 0 {
                        -1
                    } else {
                        1
                    };

                    self.reposition()
                }
            }

            numeric_api!($typ, 1);
        }
    }
}

macro_rules! unsigned_integer_bin_search {
    ($typ:ident) => {
        #[allow(missing_docs)]
        pub mod $typ {
            use std::ops::{Range, RangeFrom, RangeTo};

            use rand::{self, Rng};
            use rand::distributions::IndependentSample;

            use strategy::*;
            use test_runner::TestRunner;

            /// Shrinks an integer towards 0, using binary search to find
            /// boundary points.
            #[derive(Clone, Copy, Debug)]
            pub struct BinarySearch {
                lo: $typ,
                curr: $typ,
                hi: $typ,
            }
            impl BinarySearch {
                /// Creates a new binary searcher starting at the given value.
                pub fn new(start: $typ) -> Self {
                    BinarySearch {
                        lo: 0,
                        curr: start,
                        hi: start,
                    }
                }

                /// Creates a new binary searcher which will not search below
                /// the given `lo` value.
                fn new_clamped(lo: $typ, start: $typ, _hi: $typ) -> Self {
                    BinarySearch {
                        lo: lo,
                        curr: start,
                        hi: start,
                    }
                }

                /// Creates a new binary searcher which will not search below
                /// the given `lo` value.
                pub fn new_above(lo: $typ, start: $typ) -> Self {
                    BinarySearch::new_clamped(lo, start, start)
                }

                fn reposition(&mut self) -> bool {
                    let interval = self.hi - self.lo;
                    let new_mid = self.lo + interval/2;

                    if new_mid == self.curr {
                        false
                    } else {
                        self.curr = new_mid;
                        true
                    }
                }
            }
            impl ValueTree for BinarySearch {
                type Value = $typ;

                fn current(&self) -> $typ {
                    self.curr
                }

                fn simplify(&mut self) -> bool {
                    if self.hi <= self.lo { return false; }

                    self.hi = self.curr;
                    self.reposition()
                }

                fn complicate(&mut self) -> bool {
                    if self.hi <= self.lo { return false; }

                    self.lo = self.curr + 1;
                    self.reposition()
                }
            }

            numeric_api!($typ, 1);
        }
    }
}

signed_integer_bin_search!(i8);
signed_integer_bin_search!(i16);
signed_integer_bin_search!(i32);
signed_integer_bin_search!(i64);
signed_integer_bin_search!(isize);
unsigned_integer_bin_search!(u8);
unsigned_integer_bin_search!(u16);
unsigned_integer_bin_search!(u32);
unsigned_integer_bin_search!(u64);
unsigned_integer_bin_search!(usize);

macro_rules! float_bin_search {
    ($typ:ident) => {
        #[allow(missing_docs)]
        pub mod $typ {
            use std::ops::{Range, RangeFrom, RangeTo};

            use rand::{self, Rng};
            use rand::distributions::IndependentSample;

            use strategy::*;
            use test_runner::TestRunner;

            /// Shrinks a float towards 0, using binary search to find boundary
            /// points.
            ///
            /// Non-finite values immediately shrink to 0.
            #[derive(Clone, Copy, Debug)]
            pub struct BinarySearch {
                lo: $typ,
                curr: $typ,
                hi: $typ,
            }

            impl BinarySearch {
                /// Creates a new binary searcher starting at the given value.
                pub fn new(start: $typ) -> Self {
                    BinarySearch {
                        lo: 0.0,
                        curr: start,
                        hi: start,
                    }
                }

                /// Creates a new binary searcher which will not produce values
                /// on the other side of `lo` or `hi` from `start`. `lo` is
                /// inclusive, `hi` is exclusive.
                fn new_clamped(lo: $typ, start: $typ, hi: $typ) -> Self {
                    BinarySearch {
                        lo: if start.is_sign_negative() {
                            hi.min(0.0)
                        } else {
                            lo.max(0.0)
                        },
                        hi: start,
                        curr: start,
                    }
                }


                fn reposition(&mut self) -> bool {
                    let interval = self.hi - self.lo;
                    let interval = if interval.is_finite() {
                        interval
                    } else {
                        0.0
                    };
                    let new_mid = self.lo + interval/2.0;

                    let new_mid = if new_mid == self.curr || 0.0 == interval {
                        new_mid
                    } else {
                        self.lo
                    };

                    if new_mid == self.curr {
                        false
                    } else {
                        self.curr = new_mid;
                        true
                    }
                }

                fn done(lo: $typ, hi: $typ) -> bool {
                    (lo.abs() > hi.abs() && !hi.is_nan()) || lo.is_nan()
                }
            }
            impl ValueTree for BinarySearch {
                type Value = $typ;

                fn current(&self) -> $typ {
                    self.curr
                }

                fn simplify(&mut self) -> bool {
                    if BinarySearch::done(self.lo, self.hi) {
                        return false;
                    }

                    self.hi = self.curr;
                    self.reposition()
                }

                fn complicate(&mut self) -> bool {
                    if BinarySearch::done(self.lo, self.hi) {
                        return false;
                    }

                    if self.curr == self.lo {
                        self.lo = self.hi;
                    } else {
                        self.lo = self.curr;
                    }

                    self.reposition()
                }
            }

            numeric_api!($typ, 0.0);
        }
    }
}

float_bin_search!(f32);
float_bin_search!(f64);

#[cfg(test)]
mod test {
    use strategy::*;
    use test_runner::*;

    use super::*;

    #[test]
    fn i8_binary_search_always_converges() {
        fn assert_converges<P : Fn (i32) -> bool>(start: i8, pass: P) {
            let mut state = i8::BinarySearch::new(start);
            loop {
                if !pass(state.current() as i32) {
                    if !state.simplify() {
                        break;
                    }
                } else {
                    if !state.complicate() {
                        break;
                    }
                }
            }

            assert!(!pass(state.current() as i32));
            assert!(pass(state.current() as i32 - 1) ||
                    pass(state.current() as i32 + 1));
        }

        for start in -128..0 {
            for target in start+1..1 {
                assert_converges(start as i8, |v| v > target);
            }
        }

        for start in 0..128 {
            for target in 0..start {
                assert_converges(start as i8, |v| v < target);
            }
        }
    }

    #[test]
    fn u8_binary_search_always_converges() {
        fn assert_converges<P : Fn (u32) -> bool>(start: u8, pass: P) {
            let mut state = u8::BinarySearch::new(start);
            loop {
                if !pass(state.current() as u32) {
                    if !state.simplify() {
                        break;
                    }
                } else {
                    if !state.complicate() {
                        break;
                    }
                }
            }

            assert!(!pass(state.current() as u32));
            assert!(pass(state.current() as u32 - 1));
        }

        for start in 0..255 {
            for target in 0..start {
                assert_converges(start as u8, |v| v <= target);
            }
        }
    }

    #[test]
    fn signed_integer_range_including_zero_converges_to_zero() {
        let mut runner = TestRunner::default();
        for _ in 0..100 {
            let mut state = (-42i32..64i32).new_value(&mut runner).unwrap();
            let init_value = state.current();
            assert!(init_value >= -42 && init_value < 64);

            while state.simplify() {
                let v = state.current();
                assert!(v >= -42 && v < 64);
            }

            assert_eq!(0, state.current());
        }
    }

    #[test]
    fn negative_integer_range_stays_in_bounds() {
        let mut runner = TestRunner::default();
        for _ in 0..100 {
            let mut state = (..-42i32).new_value(&mut runner).unwrap();
            let init_value = state.current();
            assert!(init_value < -42);

            while state.simplify() {
                assert!(state.current() < -42,
                        "Violated bounds: {}", state.current());
            }

            assert_eq!(-43, state.current());
        }
    }

    #[test]
    fn positive_signed_integer_range_stays_in_bounds() {
        let mut runner = TestRunner::default();
        for _ in 0..100 {
            let mut state = (42i32..).new_value(&mut runner).unwrap();
            let init_value = state.current();
            assert!(init_value >= 42);

            while state.simplify() {
                assert!(state.current() >= 42,
                        "Violated bounds: {}", state.current());
            }

            assert_eq!(42, state.current());
        }
    }

    #[test]
    fn unsigned_integer_range_stays_in_bounds() {
        let mut runner = TestRunner::default();
        for _ in 0..100 {
            let mut state = (42u32..56u32).new_value(&mut runner).unwrap();
            let init_value = state.current();
            assert!(init_value >= 42 && init_value < 56);

            while state.simplify() {
                assert!(state.current() >= 42,
                        "Violated bounds: {}", state.current());
            }

            assert_eq!(42, state.current());
        }
    }

    #[test]
    fn unsigned_integer_binsearch_simplify_complicate_contract_upheld() {
        check_strategy_sanity(0u32..1000u32, None);
        check_strategy_sanity(0u32..1u32, None);
    }

    #[test]
    fn signed_integer_binsearch_simplify_complicate_contract_upheld() {
        check_strategy_sanity(0i32..1000i32, None);
        check_strategy_sanity(0i32..1i32, None);
    }

    #[test]
    fn positive_float_simplifies_to_zero() {
        let mut runner = TestRunner::default();
        let mut value = (0.0f64..2.0).new_value(&mut runner).unwrap();

        while value.simplify() { }

        assert_eq!(0.0, value.current());
    }

    #[test]
    fn positive_float_simplifies_to_base() {
        let mut runner = TestRunner::default();
        let mut value = (1.0f64..2.0).new_value(&mut runner).unwrap();

        while value.simplify() { }

        assert_eq!(1.0, value.current());
    }

    #[test]
    fn negative_float_simplifies_to_zero() {
        let mut runner = TestRunner::default();
        let mut value = (-2.0f64..0.0).new_value(&mut runner).unwrap();

        while value.simplify() { }

        assert_eq!(0.0, value.current());
    }

    #[test]
    fn positive_float_complicates_to_original() {
        let mut runner = TestRunner::default();
        let mut value = (1.0f64..2.0).new_value(&mut runner).unwrap();
        let orig = value.current();

        assert!(value.simplify());
        while value.complicate() { }

        assert_eq!(orig, value.current());
    }

    #[test]
    fn positive_infinity_simplifies_directly_to_zero() {
        let mut value = f64::BinarySearch::new(::std::f64::INFINITY);

        assert!(value.simplify());
        assert_eq!(0.0, value.current());
        assert!(value.complicate());
        assert_eq!(::std::f64::INFINITY, value.current());
        assert!(!value.clone().complicate());
        assert!(!value.clone().simplify());
    }

    #[test]
    fn negative_infinity_simplifies_directly_to_zero() {
        let mut value = f64::BinarySearch::new(::std::f64::NEG_INFINITY);

        assert!(value.simplify());
        assert_eq!(0.0, value.current());
        assert!(value.complicate());
        assert_eq!(::std::f64::NEG_INFINITY, value.current());
        assert!(!value.clone().complicate());
        assert!(!value.clone().simplify());
    }

    #[test]
    fn nan_simplifies_directly_to_zero() {
        let mut value = f64::BinarySearch::new(::std::f64::NAN);

        assert!(value.simplify());
        assert_eq!(0.0, value.current());
        assert!(value.complicate());
        assert!(value.current().is_nan());
        assert!(!value.clone().complicate());
        assert!(!value.clone().simplify());
    }

    #[test]
    fn float_simplifies_to_smallest_normal() {
        let mut runner = TestRunner::default();
        let mut value = (::std::f64::MIN_POSITIVE..2.0)
            .new_value(&mut runner).unwrap();

        while value.simplify() { }

        assert_eq!(::std::f64::MIN_POSITIVE, value.current());
    }
}
