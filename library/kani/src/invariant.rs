// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module introduces the `Invariant` trait as well as its implementation
//! for primitive types.

/// This trait should be used to specify and check type safety invariants for a
/// type. For type invariants, we refer to the definitions in the Rust's Unsafe
/// Code Guidelines Reference:
/// <https://rust-lang.github.io/unsafe-code-guidelines/glossary.html#validity-and-safety-invariant>
///
/// In summary, the reference distinguishes two kinds of type invariants:
///  - *Validity invariant*: An invariant that all data must uphold any time
///    it's accessed or copied in a typed manner. This invariant is exploited by
///    the compiler to perform optimizations.
///  - *Safety invariant*: An invariant that safe code may assume all data to
///    uphold. This invariant can be temporarily violated by unsafe code, but
///    must always be upheld when interfacing with unknown safe code.
///
/// Therefore, validity invariants must be upheld at all times, while safety
/// invariants only need to be upheld at the boundaries to safe code.
///
/// Safety invariants are particularly interesting for user-defined types, and
/// the `Invariant` trait allows you to check them with Kani.
///
/// It can also be used in tests. It's a programmatic way to specify (in Rust)
/// properties over your data types. Since it's written in Rust, it can be used
/// for static and dynamic checking.
///
/// For example, let's say you're creating a type that represents a date:
///
/// ```rust
/// #[derive(kani::Arbitrary)]
/// pub struct MyDate {
///   day: u8,
///   month: u8,
///   year: i64,
/// }
/// ```
/// You can specify its safety invariant as:
/// ```rust
/// impl kani::Invariant for MyDate {
///   fn is_safe(&self) -> bool {
///     self.month > 0
///       && self.month <= 12
///       && self.day > 0
///       && self.day <= days_in_month(self.year, self.month)
///   }
/// }
/// ```
/// And use it to check that your APIs are safe:
/// ```rust
/// #[kani::proof]
/// fn check_increase_date() {
///   let mut date: MyDate = kani::any();
///   // Increase date by one day
///   increase_date(date, 1);
///   assert!(date.is_safe());
/// }
/// ```
pub trait Invariant
where
    Self: Sized,
{
    fn is_safe(&self) -> bool;
}

/// Any value is considered safe for the type
macro_rules! trivial_invariant {
    ( $type: ty ) => {
        impl Invariant for $type {
            #[inline(always)]
            fn is_safe(&self) -> bool {
                true
            }
        }
    };
}

trivial_invariant!(u8);
trivial_invariant!(u16);
trivial_invariant!(u32);
trivial_invariant!(u64);
trivial_invariant!(u128);
trivial_invariant!(usize);

trivial_invariant!(i8);
trivial_invariant!(i16);
trivial_invariant!(i32);
trivial_invariant!(i64);
trivial_invariant!(i128);
trivial_invariant!(isize);

// We do not constrain the safety invariant for floating points types.
// Users can create a new type wrapping the floating point type and define an
// invariant that checks for NaN, infinite, or subnormal values.
trivial_invariant!(f32);
trivial_invariant!(f64);

trivial_invariant!(());
trivial_invariant!(bool);
trivial_invariant!(char);
