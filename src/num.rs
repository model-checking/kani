//-
// Copyright 2017, 2018 Jason Lingle
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

macro_rules! int_any {
    () => {
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
    }
}

macro_rules! numeric_api {
    ($typ:ident, $epsilon:expr) => {
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

            int_any!();

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

            int_any!();

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

#[cfg(feature = "unstable")]
signed_integer_bin_search!(i128);
#[cfg(feature = "unstable")]
unsigned_integer_bin_search!(u128);

bitflags! {
    pub(crate) struct FloatTypes: u32 {
        const POSITIVE          = 0b0000001;
        const NEGATIVE          = 0b0000010;
        const NORMAL            = 0b0000100;
        const SUBNORMAL         = 0b0001000;
        const ZERO              = 0b0010000;
        const INFINITE          = 0b0100000;
        const QUIET_NAN         = 0b1000000;
        const ANY =
            Self::POSITIVE.bits |
            Self::NEGATIVE.bits |
            Self::NORMAL.bits |
            Self::SUBNORMAL.bits |
            Self::ZERO.bits |
            Self::INFINITE.bits |
            Self::QUIET_NAN.bits;
    }
}

impl FloatTypes {
    fn normalise(mut self) -> Self {
        if !self.intersects(FloatTypes::POSITIVE | FloatTypes::NEGATIVE) {
            self |= FloatTypes::POSITIVE;
        }

        if !self.intersects(FloatTypes::NORMAL | FloatTypes::SUBNORMAL |
                            FloatTypes::ZERO | FloatTypes::INFINITE |
                            FloatTypes::QUIET_NAN) {
            self |= FloatTypes::NORMAL;
        }
        self
    }
}

trait FloatLayout {
    type Bits : ::rand::Rand + Copy;

    const SIGN_MASK: Self::Bits;
    const EXP_MASK: Self::Bits;
    const EXP_ZERO: Self::Bits;
    const MANTISSA_MASK: Self::Bits;
}

impl FloatLayout for f32 {
    type Bits = u32;

    const SIGN_MASK: u32        = 0x8000_0000;
    const EXP_MASK: u32         = 0x7F80_0000;
    const EXP_ZERO: u32         = 0x3F80_0000;
    const MANTISSA_MASK: u32    = 0x007F_FFFF;
}

impl FloatLayout for f64 {
    type Bits = u64;

    const SIGN_MASK: u64        = 0x8000_0000_0000_0000;
    const EXP_MASK: u64         = 0x7FF0_0000_0000_0000;
    const EXP_ZERO: u64         = 0x3FF0_0000_0000_0000;
    const MANTISSA_MASK: u64    = 0x000F_FFFF_FFFF_FFFF;
}

macro_rules! float_any {
    ($typ:ident) => {
        /// Strategies which produce floating-point values from particular
        /// classes. See the various `Any`-typed constants in this module.
        ///
        /// Note that this usage is fairly advanced and primarily useful to
        /// implementors of algorithms that need to handle wild values in a
        /// particular way. For testing things like graphics processing or game
        /// physics, simply using ranges (e.g., `-1.0..2.0`) will often be more
        /// practical.
        ///
        /// `Any` can be OR'ed to combine multiple classes. For example,
        /// `POSITIVE | INFINITE` will generate arbitrary positive, non-NaN
        /// floats, including positive infinity (but not negative infinity, of
        /// course).
        ///
        /// If neither `POSITIVE` nor `NEGATIVE` has been OR'ed into an `Any`
        /// but a type to be generated requires a sign, `POSITIVE` is assumed.
        /// If no classes are OR'ed into an `Any` (i.e., only `POSITIVE` and/or
        /// `NEGATIVE` are given), `NORMAL` is assumed.
        ///
        /// The various float classes are assigned fixed weights for generation
        /// which are believed to be reasonable for most applications. Roughly:
        ///
        /// - If `POSITIE | NEGATIVE`, the sign is evenly distributed between
        ///   both options.
        ///
        /// - Classes are weighted as follows, in descending order:
        ///   `NORMAL` > `ZERO` > `SUBNORMAL` > `INFINITE` > `QUIET_NAN`.
        #[derive(Clone, Copy, Debug)]
        pub struct Any(FloatTypes);

        #[cfg(test)]
        impl Any {
            pub(crate) fn from_bits(bits: u32) -> Self {
                Any(FloatTypes::from_bits_truncate(bits))
            }

            pub(crate) fn normal_bits(&self) -> FloatTypes {
                self.0.normalise()
            }
        }

        impl ops::BitOr for Any {
            type Output = Self;

            fn bitor(self, rhs: Self) -> Self {
                Any(self.0 | rhs.0)
            }
        }

        impl ops::BitOrAssign for Any {
            fn bitor_assign(&mut self, rhs: Self) {
                self.0 |= rhs.0
            }
        }

        /// Generates positive floats
        ///
        /// By itself, implies the `NORMAL` class, unless another class is
        /// OR'ed in. That is, using `POSITIVE` as a strategy by itself will
        /// generate arbitrary values between the type's `MIN_POSITIVE` and
        /// `MAX`, while `POSITIVE | INFINITE` would only allow generating
        /// positive infinity.
        pub const POSITIVE: Any = Any(FloatTypes::POSITIVE);
        /// Generates negative floats.
        ///
        /// By itself, implies the `NORMAL` class, unless another class is
        /// OR'ed in. That is, using `POSITIVE` as a strategy by itself will
        /// generate arbitrary values between the type's `MIN` and
        /// `-MIN_POSITIVE`, while `NEGATIVE | INFINITE` would only allow
        /// generating positive infinity.
        pub const NEGATIVE: Any = Any(FloatTypes::NEGATIVE);
        /// Generates "normal" floats.
        ///
        /// These are finite values where the first bit of the mantissa is an
        /// implied `1`. When positive, this represents the range
        /// `MIN_POSITIVE` through `MAX`, both inclusive.
        ///
        /// Generated values are uniform over the discrete floating-point
        /// space, which means the numeric distribution is an inverse
        /// exponential step function. For example, values between 1.0 and 2.0
        /// are generated with the same frequency as values between 2.0 and
        /// 4.0, even though the latter covers twice the numeric range.
        ///
        /// If neither `POSITIVE` nor `NEGATIVE` is OR'ed with this constant,
        /// `POSITIVE` is implied.
        pub const NORMAL: Any = Any(FloatTypes::NORMAL);
        /// Generates subnormal floats.
        ///
        /// These are finite non-zero values where the first bit of the
        /// mantissa is not an implied zero. When positive, this represents the
        /// range `MIN`, inclusive, through `MIN_POSITIVE`, exclusive.
        ///
        /// Subnormals are generated with a uniform distribution both in terms
        /// of discrete floating-point space and numerically.
        ///
        /// If neither `POSITIVE` nor `NEGATIVE` is OR'ed with this constant,
        /// `POSITIVE` is implied.
        pub const SUBNORMAL: Any = Any(FloatTypes::SUBNORMAL);
        /// Generates zero-valued floats.
        ///
        /// Note that IEEE floats support both positive and negative zero, so
        /// this class does interact with the sign flags.
        ///
        /// If neither `POSITIVE` nor `NEGATIVE` is OR'ed with this constant,
        /// `POSITIVE` is implied.
        pub const ZERO: Any = Any(FloatTypes::ZERO);
        /// Generates infinity floats.
        ///
        /// If neither `POSITIVE` nor `NEGATIVE` is OR'ed with this constant,
        /// `POSITIVE` is implied.
        pub const INFINITE: Any = Any(FloatTypes::INFINITE);
        /// Generates "Quiet NaN" floats.
        ///
        /// Operations on quiet NaNs generally simply propagate the NaN rather
        /// than invoke any exception mechanism.
        ///
        /// Note that there is currently no `SIGNALING_NAN` is the question of
        /// whether generation of signaling NaNs is safe is [currently
        /// uncertain](https://doc.rust-lang.org/stable/std/primitive.f32.html#method.from_bits).
        ///
        /// The payload of the NaN is uniformly distributed over the possible
        /// values which safe Rust allows, including the sign bit (as
        /// controlled by `POSITIVE` and `NEGATIVE`). Note however that as of
        /// Rust 1.23.0, this constitutes only one particular payload due to
        /// apparent issues with particular MIPS and PA-RISC processors which
        /// fail to implement IEEE 754-2008 correctly.
        pub const QUIET_NAN: Any = Any(FloatTypes::QUIET_NAN);

        /// Generates literally arbitrary floating-point values, including
        /// infinities and quiet NaNs (but not signaling NaNs).
        ///
        /// Equivalent to `POSITIVE | NEGATIVE | NORMAL | SUBNORMAL | ZERO |
        /// INFINITE | QUIET_NAN`.
        ///
        /// Before proptest 0.4.1, this erroneously generated values in the
        /// range 0.0..1.0.
        pub const ANY: Any = Any(FloatTypes::ANY);

        impl Strategy for Any {
            type Value = BinarySearch;

            fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
                let flags = self.0.normalise();
                let sign_mask = if flags.contains(FloatTypes::NEGATIVE) {
                    $typ::SIGN_MASK
                } else {
                    0
                };
                let sign_or = if flags.contains(FloatTypes::POSITIVE) {
                    0
                } else {
                    $typ::SIGN_MASK
                };

                macro_rules! weight {
                    ($case:ident, $weight:expr) => {
                        if flags.contains(FloatTypes::$case) {
                            $weight
                        } else {
                            0
                        }
                    }
                }

                let (class_mask, class_or, allow_edge_exp, allow_zero_mant) =
                    prop_oneof![
                        weight!(NORMAL, 20) => Just(
                            ($typ::EXP_MASK | $typ::MANTISSA_MASK, 0,
                             false, true)),
                        weight!(SUBNORMAL, 3) => Just(
                            ($typ::MANTISSA_MASK, 0, true, false)),
                        weight!(ZERO, 4) => Just(
                            (0, 0, true, true)),
                        weight!(INFINITE, 2) => Just(
                            (0, $typ::EXP_MASK, true, true)),
                        weight!(QUIET_NAN, 1) => Just(
                            ($typ::MANTISSA_MASK >> 1, $typ::EXP_MASK,
                             true, false)),
                    ].new_value(runner)?.current();

                let mut generated_value: <$typ as FloatLayout>::Bits =
                    runner.rng().gen();
                generated_value &= sign_mask | class_mask;
                generated_value |= sign_or | class_or;
                let exp = generated_value & $typ::EXP_MASK;
                if !allow_edge_exp && (0 == exp || $typ::EXP_MASK == exp) {
                    generated_value &= !$typ::EXP_MASK;
                    generated_value |= $typ::EXP_ZERO;
                }
                if !allow_zero_mant &&
                    0 == generated_value & $typ::MANTISSA_MASK
                {
                    generated_value |= 1;
                }

                Ok(BinarySearch::new_with_types(
                    $typ::from_bits(generated_value), flags))
            }
        }
    }
}

macro_rules! float_bin_search {
    ($typ:ident) => {
        #[allow(missing_docs)]
        pub mod $typ {
            use std::ops::{self, Range, RangeFrom, RangeTo};

            use rand::{self, Rng};
            use rand::distributions::IndependentSample;

            use strategy::*;
            use test_runner::TestRunner;
            use super::{FloatTypes, FloatLayout};

            float_any!($typ);

            /// Shrinks a float towards 0, using binary search to find boundary
            /// points.
            ///
            /// Non-finite values immediately shrink to 0.
            #[derive(Clone, Copy, Debug)]
            pub struct BinarySearch {
                lo: $typ,
                curr: $typ,
                hi: $typ,
                allowed: FloatTypes,
            }

            impl BinarySearch {
                /// Creates a new binary searcher starting at the given value.
                pub fn new(start: $typ) -> Self {
                    BinarySearch {
                        lo: 0.0,
                        curr: start,
                        hi: start,
                        allowed: FloatTypes::all(),
                    }
                }

                fn new_with_types(start: $typ, allowed: FloatTypes) -> Self {
                    BinarySearch {
                        lo: 0.0,
                        curr: start,
                        hi: start,
                        allowed,
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
                        allowed: FloatTypes::all(),
                    }
                }

                fn current_allowed(&self) -> bool {
                    use std::num::FpCategory;

                    // Don't reposition if the new value is not allowed
                    let class_allowed = match self.curr.classify() {
                        FpCategory::Nan =>
                            self.allowed.contains(FloatTypes::QUIET_NAN),
                        FpCategory::Infinite =>
                            self.allowed.contains(FloatTypes::INFINITE),
                        FpCategory::Zero =>
                            self.allowed.contains(FloatTypes::ZERO),
                        FpCategory::Subnormal =>
                            self.allowed.contains(FloatTypes::SUBNORMAL),
                        FpCategory::Normal =>
                            self.allowed.contains(FloatTypes::NORMAL),
                    };
                    let signum = self.curr.signum();
                    let sign_allowed = if signum > 0.0 {
                        self.allowed.contains(FloatTypes::POSITIVE)
                    } else if signum < 0.0 {
                        self.allowed.contains(FloatTypes::NEGATIVE)
                    } else {
                        true
                    };

                    class_allowed && sign_allowed
                }

                fn ensure_acceptable(&mut self) {
                    while !self.current_allowed() {
                        if !self.complicate_once() {
                            panic!("Unable to complicate floating-point back \
                                    to acceptable value");
                        }
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

                fn complicate_once(&mut self) -> bool {
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
                    if self.reposition() {
                        self.ensure_acceptable();
                        true
                    } else {
                        false
                    }
                }

                fn complicate(&mut self) -> bool {
                    if self.complicate_once() {
                        self.ensure_acceptable();
                        true
                    } else {
                        false
                    }
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

    macro_rules! float_generation_test_body {
        ($strategy:ident) => {
            use std::num::FpCategory;

            let strategy = $strategy;
            let bits = strategy.normal_bits();

            let mut seen_positive = 0;
            let mut seen_negative = 0;
            let mut seen_normal = 0;
            let mut seen_subnormal = 0;
            let mut seen_zero = 0;
            let mut seen_infinite = 0;
            let mut seen_nan = 0;
            let mut runner = TestRunner::default();

            for _ in 0..1024 {
                let mut tree = strategy.new_value(&mut runner).unwrap();
                let mut increment = 1;

                loop {
                    let value = tree.current();

                    let sign = value.signum(); // So we correctly handle -0
                    if sign < 0.0 {
                        prop_assert!(bits.contains(FloatTypes::NEGATIVE));
                        seen_negative += increment;
                    } else if sign > 0.0 { // i.e., not NaN
                        prop_assert!(bits.contains(FloatTypes::POSITIVE));
                        seen_positive += increment;
                    }

                    match value.classify() {
                        FpCategory::Nan => {
                            prop_assert!(bits.contains(FloatTypes::QUIET_NAN));
                            seen_nan += increment;
                            // Since safe Rust doesn't currently allow generating
                            // any NaN other than one particular payload, don't
                            // check the sign and consider this to be both signs
                            // for counting purposes.
                            seen_positive += increment;
                            seen_negative += increment;
                        },
                        FpCategory::Infinite => {
                            prop_assert!(bits.contains(FloatTypes::INFINITE));
                            seen_infinite += increment;
                        },
                        FpCategory::Zero => {
                            prop_assert!(bits.contains(FloatTypes::ZERO));
                            seen_zero += increment;
                        },
                        FpCategory::Subnormal => {
                            prop_assert!(bits.contains(FloatTypes::SUBNORMAL));
                            seen_subnormal += increment;
                        },
                        FpCategory::Normal => {
                            prop_assert!(bits.contains(FloatTypes::NORMAL));
                            seen_normal += increment;
                        },
                    }

                    // Don't count simplified values towards the counts
                    increment = 0;
                    if !tree.simplify() { break; }
                }
            }

            if bits.contains(FloatTypes::POSITIVE) {
                prop_assert!(seen_positive > 200);
            }
            if bits.contains(FloatTypes::NEGATIVE) {
                prop_assert!(seen_negative > 200);
            }
            if bits.contains(FloatTypes::NORMAL) {
                prop_assert!(seen_normal > 100);
            }
            if bits.contains(FloatTypes::SUBNORMAL) {
                prop_assert!(seen_subnormal > 5);
            }
            if bits.contains(FloatTypes::ZERO) {
                prop_assert!(seen_zero > 5);
            }
            if bits.contains(FloatTypes::INFINITE) {
                prop_assert!(seen_infinite > 0);
            }
            if bits.contains(FloatTypes::QUIET_NAN) {
                prop_assert!(seen_nan > 0);
            }
        }
    }

    proptest! {
        #![proptest_config(::test_runner::Config::with_cases(1024))]

        #[test]
        fn f32_any_generates_desired_values(
            strategy in ::bits::u32::ANY.prop_map(f32::Any::from_bits)
        ) {
            float_generation_test_body!(strategy);
        }

        #[test]
        fn f32_any_sanity(
            strategy in ::bits::u32::ANY.prop_map(f32::Any::from_bits)
        ) {
            check_strategy_sanity(strategy, Some(CheckStrategySanityOptions {
                strict_complicate_after_simplify: false,
                .. CheckStrategySanityOptions::default()
            }));
        }

        #[test]
        fn f64_any_generates_desired_values(
            strategy in ::bits::u32::ANY.prop_map(f64::Any::from_bits)
        ) {
            float_generation_test_body!(strategy);
        }

        #[test]
        fn f64_any_sanity(
            strategy in ::bits::u32::ANY.prop_map(f64::Any::from_bits)
        ) {
            check_strategy_sanity(strategy, Some(CheckStrategySanityOptions {
                strict_complicate_after_simplify: false,
                .. CheckStrategySanityOptions::default()
            }));
        }
    }
}
