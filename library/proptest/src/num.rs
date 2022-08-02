//-
// Copyright 2017, 2018 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
// Modifications Copyright Kani Contributors
// See GitHub history for details

//! Strategies to generate numeric values (as opposed to integers used as bit
//! fields).
//!
//! All strategies in this module shrink by binary searching towards 0.

use crate::test_runner::TestRunner;
use core::ops::Range;

// Below 2 functions are the source of Kani symbolic variables.

/// Produce an symbolic value from a range.
pub(crate) fn sample_uniform<X: kani::Arbitrary + PartialOrd>(_: &mut TestRunner, range: Range<X>) -> X {
    let value: X = kani::any();
    kani::assume(range.contains(&value));
    value
}

/// Produce an symbolic value start and end values. End is inclusive.
pub(crate) fn sample_uniform_incl<X: kani::Arbitrary + PartialOrd>(_: &mut TestRunner, start: X, end: X) -> X {
    let value: X = kani::any();
    kani::assume(value <= end);
    kani::assume(value >= start);
    value
}

macro_rules! int_any {
    ($typ: ident) => {
        /// Type of the `ANY` constant.
        #[derive(Clone, Copy, Debug)]
        #[must_use = "strategies do nothing unless used"]
        pub struct Any(());
        /// Generates integers with completely arbitrary values, uniformly
        /// distributed over the whole range.
        pub const ANY: Any = Any(());

        impl Strategy for Any {
            type Tree = BinarySearch;
            type Value = $typ;

            fn new_tree(&self, _: &mut TestRunner) -> NewTree<Self> {
                Ok(BinarySearch::new(kani::any::<$typ>()))
            }
        }
    };
}

macro_rules! numeric_api {
    ($typ:ident, $epsilon:expr) => {
        impl Strategy for ::core::ops::Range<$typ> {
            type Tree = BinarySearch;
            type Value = $typ;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(BinarySearch::new_clamped(
                    self.start,
                    $crate::num::sample_uniform(runner, self.clone()),
                    self.end - $epsilon,
                ))
            }
        }

        impl Strategy for ::core::ops::RangeInclusive<$typ> {
            type Tree = BinarySearch;
            type Value = $typ;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(BinarySearch::new_clamped(
                    *self.start(),
                    $crate::num::sample_uniform_incl(runner, *self.start(), *self.end()),
                    *self.end(),
                ))
            }
        }

        impl Strategy for ::core::ops::RangeFrom<$typ> {
            type Tree = BinarySearch;
            type Value = $typ;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(BinarySearch::new_clamped(
                    self.start,
                    $crate::num::sample_uniform_incl(runner, self.start, ::core::$typ::MAX),
                    ::core::$typ::MAX,
                ))
            }
        }

        impl Strategy for ::core::ops::RangeTo<$typ> {
            type Tree = BinarySearch;
            type Value = $typ;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(BinarySearch::new_clamped(
                    ::core::$typ::MIN,
                    $crate::num::sample_uniform(runner, ::core::$typ::MIN..self.end),
                    self.end,
                ))
            }
        }

        impl Strategy for ::core::ops::RangeToInclusive<$typ> {
            type Tree = BinarySearch;
            type Value = $typ;

            fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
                Ok(BinarySearch::new_clamped(
                    ::core::$typ::MIN,
                    $crate::num::sample_uniform_incl(runner, ::core::$typ::MIN, self.end),
                    self.end,
                ))
            }
        }
    };
}

macro_rules! signed_integer_bin_search {
    ($typ:ident) => {
        #[allow(missing_docs)]
        pub mod $typ {

            use crate::strategy::*;
            use crate::test_runner::TestRunner;

            int_any!($typ);

            /// Shrinks an integer towards 0, using binary search to find
            /// boundary points.
            #[derive(Clone, Copy, Debug)]
            pub struct BinarySearch {
                curr: $typ,
            }
            impl BinarySearch {
                /// Creates a new binary searcher starting at the given value.
                pub fn new(start: $typ) -> Self {
                    BinarySearch { curr: start, }
                }

                /// Creates a new binary searcher which will not produce values
                /// on the other side of `lo` or `hi` from `start`. `lo` is
                /// inclusive, `hi` is exclusive.
                fn new_clamped(_: $typ, start: $typ, _: $typ) -> Self {
                    BinarySearch {
                        curr: start,
                    }
                }
            }
            impl ValueTree for BinarySearch {
                type Value = $typ;

                fn current(&self) -> $typ {
                    self.curr
                }

                fn simplify(&mut self) -> bool {
                    return false;
                }

                fn complicate(&mut self) -> bool {
                    return false;
                }
            }

            numeric_api!($typ, 1);
        }
    };
}

macro_rules! unsigned_integer_bin_search {
    ($typ:ident) => {
        #[allow(missing_docs)]
        pub mod $typ {

            use crate::strategy::*;
            use crate::test_runner::TestRunner;

            int_any!($typ);

            /// Shrinks an integer towards 0, using binary search to find
            /// boundary points.
            #[derive(Clone, Copy, Debug)]
            pub struct BinarySearch {
                curr: $typ,
            }
            impl BinarySearch {
                /// Creates a new binary searcher starting at the given value.
                pub fn new(start: $typ) -> Self {
                    BinarySearch { curr: start, }
                }

                /// Creates a new binary searcher which will not search below
                /// the given `lo` value.
                fn new_clamped(_: $typ, start: $typ, _: $typ) -> Self {
                    BinarySearch { curr: start, }
                }

                /// Creates a new binary searcher which will not search below
                /// the given `lo` value.
                pub fn new_above(lo: $typ, start: $typ) -> Self {
                    BinarySearch::new_clamped(lo, start, start)
                }
            }
            impl ValueTree for BinarySearch {
                type Value = $typ;

                fn current(&self) -> $typ {
                    self.curr
                }

                fn simplify(&mut self) -> bool {
                    return false;
                }

                fn complicate(&mut self) -> bool {
                    return false;
                }
            }

            numeric_api!($typ, 1);
        }
    };
}

signed_integer_bin_search!(i8);
signed_integer_bin_search!(i16);
signed_integer_bin_search!(i32);
signed_integer_bin_search!(i64);
#[cfg(not(target_arch = "wasm32"))]
signed_integer_bin_search!(i128);
signed_integer_bin_search!(isize);
unsigned_integer_bin_search!(u8);
unsigned_integer_bin_search!(u16);
unsigned_integer_bin_search!(u32);
unsigned_integer_bin_search!(u64);
#[cfg(not(target_arch = "wasm32"))]
unsigned_integer_bin_search!(u128);
unsigned_integer_bin_search!(usize);

#[derive(Clone, Copy, Debug)]
pub(crate) struct  FloatTypes(u32);

impl std::ops::BitOr for FloatTypes {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for FloatTypes {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl FloatTypes {
    const POSITIVE       : FloatTypes = FloatTypes(0b0000_0001);
    const NEGATIVE       : FloatTypes = FloatTypes(0b0000_0010);
    const NORMAL         : FloatTypes = FloatTypes(0b0000_0100);
    const SUBNORMAL      : FloatTypes = FloatTypes(0b0000_1000);
    const ZERO           : FloatTypes = FloatTypes(0b0001_0000);
    const INFINITE       : FloatTypes = FloatTypes(0b0010_0000);
    const QUIET_NAN      : FloatTypes = FloatTypes(0b0100_0000);
    const SIGNALING_NAN  : FloatTypes = FloatTypes(0b1000_0000);
    const ANY            : FloatTypes = FloatTypes(0b1111_1111);

    fn intersects(&self , other: Self) -> bool {
        let intersection = self.0 & other.0;
        intersection != 0
    }

    fn contains(&self , other: Self) -> bool {
        let intersection = self.0 & other.0;
        intersection == other.0
    }

    fn normalise(mut self) -> Self {
        if !self.intersects(FloatTypes::POSITIVE | FloatTypes::NEGATIVE) {
            self |= FloatTypes::POSITIVE;
        }

        if !self.intersects(
            FloatTypes::NORMAL
                | FloatTypes::SUBNORMAL
                | FloatTypes::ZERO
                | FloatTypes::INFINITE
                | FloatTypes::QUIET_NAN
                | FloatTypes::SIGNALING_NAN,
        ) {
            self |= FloatTypes::NORMAL;
        }
        self
    }
}

trait FloatLayout
{
    type Bits: Copy;

    const SIGN_MASK: Self::Bits;
    const EXP_MASK: Self::Bits;
    const EXP_ZERO: Self::Bits;
    const MANTISSA_MASK: Self::Bits;
}

impl FloatLayout for f32 {
    type Bits = u32;

    const SIGN_MASK: u32 = 0x8000_0000;
    const EXP_MASK: u32 = 0x7F80_0000;
    const EXP_ZERO: u32 = 0x3F80_0000;
    const MANTISSA_MASK: u32 = 0x007F_FFFF;
}

impl FloatLayout for f64 {
    type Bits = u64;

    const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
    const EXP_MASK: u64 = 0x7FF0_0000_0000_0000;
    const EXP_ZERO: u64 = 0x3FF0_0000_0000_0000;
    const MANTISSA_MASK: u64 = 0x000F_FFFF_FFFF_FFFF;
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
        /// - If `POSITIVE | NEGATIVE`, the sign is evenly distributed between
        ///   both options.
        ///
        /// - Classes are weighted as follows, in descending order:
        ///   `NORMAL` > `ZERO` > `SUBNORMAL` > `INFINITE` > `QUIET_NAN` =
        ///   `SIGNALING_NAN`.
        #[derive(Clone, Copy, Debug)]
        #[must_use = "strategies do nothing unless used"]
        pub struct Any(FloatTypes);

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
        /// The payload of the NaN is uniformly distributed over the possible
        /// values which safe Rust allows, including the sign bit (as
        /// controlled by `POSITIVE` and `NEGATIVE`).
        ///
        /// Note however that in Rust 1.23.0 and earlier, this constitutes only
        /// one particular payload due to apparent issues with particular MIPS
        /// and PA-RISC processors which fail to implement IEEE 754-2008
        /// correctly.
        ///
        /// On Rust 1.24.0 and later, this does produce arbitrary payloads as
        /// documented.
        ///
        /// On platforms where the CPU and the IEEE standard disagree on the
        /// format of a quiet NaN, values generated conform to the hardware's
        /// expectations.
        pub const QUIET_NAN: Any = Any(FloatTypes::QUIET_NAN);
        /// Generates "Signaling NaN" floats if allowed by the platform.
        ///
        /// On most platforms, signalling NaNs by default behave the same as
        /// quiet NaNs, but it is possible to configure the OS or CPU to raise
        /// an asynchronous exception if an operation is performed on a
        /// signalling NaN.
        ///
        /// In Rust 1.23.0 and earlier, this silently behaves the same as
        /// [`QUIET_NAN`](const.QUIET_NAN.html).
        ///
        /// On platforms where the CPU and the IEEE standard disagree on the
        /// format of a quiet NaN, values generated conform to the hardware's
        /// expectations.
        ///
        /// Note that certain platforms — most notably, x86/AMD64 — allow the
        /// architecture to turn a signalling NaN into a quiet NaN with the
        /// same payload. Whether this happens can depend on what registers the
        /// compiler decides to use to pass the value around, what CPU flags
        /// are set, and what compiler settings are in use.
        pub const SIGNALING_NAN: Any = Any(FloatTypes::SIGNALING_NAN);

        /// Generates literally arbitrary floating-point values, including
        /// infinities and quiet NaNs (but not signaling NaNs).
        ///
        /// Equivalent to `POSITIVE | NEGATIVE | NORMAL | SUBNORMAL | ZERO |
        /// INFINITE | QUIET_NAN`.
        ///
        /// See [`SIGNALING_NAN`](const.SIGNALING_NAN.html) if you also want to
        /// generate signalling NaNs. This signalling NaNs are not included by
        /// default since in most contexts they either make no difference, or
        /// if the process enabled the relevant CPU mode, result in
        /// hardware-triggered exceptions that usually just abort the process.
        ///
        /// Before proptest 0.4.1, this erroneously generated values in the
        /// range 0.0..1.0.
        pub const ANY: Any = Any(FloatTypes::ANY);

        impl Strategy for Any {
            type Tree = BinarySearch;
            type Value = $typ;

            fn new_tree(&self, _: &mut TestRunner) -> NewTree<Self> {
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

                // A few CPUs disagree with IEEE about the meaning of the
                // signalling bit. Assume the `NAN` constant is a quiet NaN as
                // interpreted by the hardware and generate values based on
                // that.
                let quiet_or = ::core::$typ::NAN.to_bits() &
                    ($typ::EXP_MASK | ($typ::EXP_MASK >> 1));
                let signaling_or = (quiet_or ^ ($typ::EXP_MASK >> 1)) |
                    $typ::EXP_MASK;

                let (class_mask, class_or, allow_edge_exp, allow_zero_mant) =
                 || -> (_, _, bool, bool) {
                    let can_match_after =
                        flags.contains(FloatTypes::SUBNORMAL)  ||
                        flags.contains(FloatTypes::ZERO)  ||
                        flags.contains(FloatTypes::INFINITE)  ||
                        flags.contains(FloatTypes::QUIET_NAN)  ||
                        flags.contains(FloatTypes::SIGNALING_NAN);
                    if flags.contains(FloatTypes::NORMAL) && (!can_match_after || kani::any()) {
                        return ($typ::EXP_MASK | $typ::MANTISSA_MASK, 0, false, true);
                    }

                    let can_match_after =
                        flags.contains(FloatTypes::ZERO)  ||
                        flags.contains(FloatTypes::INFINITE)  ||
                        flags.contains(FloatTypes::QUIET_NAN)  ||
                        flags.contains(FloatTypes::SIGNALING_NAN);
                    if flags.contains(FloatTypes::SUBNORMAL) && (!can_match_after || kani::any()) {
                        return ($typ::MANTISSA_MASK, 0, true, false)
                    }

                    let can_match_after =
                        flags.contains(FloatTypes::INFINITE)  ||
                        flags.contains(FloatTypes::QUIET_NAN)  ||
                        flags.contains(FloatTypes::SIGNALING_NAN);
                    if flags.contains(FloatTypes::ZERO) && (!can_match_after || kani::any()) {
                        return (0, 0, true, true);
                    }

                    let can_match_after =
                        flags.contains(FloatTypes::QUIET_NAN)  ||
                        flags.contains(FloatTypes::SIGNALING_NAN);
                    if flags.contains(FloatTypes::INFINITE) && (!can_match_after || kani::any()) {
                        return (0, $typ::EXP_MASK, true, true);
                    }

                    let can_match_after = flags.contains(FloatTypes::SIGNALING_NAN);
                    if flags.contains(FloatTypes::QUIET_NAN) && (!can_match_after || kani::any()) {
                        return ($typ::MANTISSA_MASK >> 1, quiet_or, true, false);
                    }

                    if flags.contains(FloatTypes::SIGNALING_NAN) {
                        return ($typ::MANTISSA_MASK >> 1, signaling_or,true, false);
                    }

                    panic!("This should not be reachable. This is a bug in Kani or Kani's proptest library")
                }();

                let mut generated_value: <$typ as FloatLayout>::Bits = kani::any();
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
            use core::ops;
            use super::{FloatLayout, FloatTypes};
            use crate::strategy::*;
            use crate::test_runner::TestRunner;

            float_any!($typ);

            /// Binary Search Strategy Modified for Kani. It does not
            /// perform any random search, but rather returns a
            /// symbolic current value.
            #[derive(Clone, Copy, Debug)]
            pub struct BinarySearch {
                curr: $typ,
            }

            impl BinarySearch {
                /// Creates a new binary searcher starting at the given value.
                pub fn new(start: $typ) -> Self {
                    BinarySearch { curr: start, }
                }

                fn new_with_types(start: $typ, _: FloatTypes) -> Self {
                    BinarySearch { curr: start, }
                }

                /// Creates a new binary searcher which will not produce values
                /// on the other side of `lo` or `hi` from `start`. `lo` is
                /// inclusive, `hi` is exclusive.
                fn new_clamped(_: $typ, start: $typ, _: $typ) -> Self {
                    BinarySearch {
                        curr: start,
                    }
                }
            }

            impl ValueTree for BinarySearch {
                type Value = $typ;

                fn current(&self) -> $typ {
                    self.curr
                }

                fn simplify(&mut self) -> bool {
                    false
                }

                fn complicate(&mut self) -> bool {
                    false
                }
            }

            numeric_api!($typ, 0.0);
        }
    };
}

float_bin_search!(f32);
float_bin_search!(f64);
