// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Useful utilities for CBMC

use crate::InternedString;
use num::Signed;
use num::bigint::{BigInt, Sign};
use num_traits::Zero;

/// The aggregate name used in CBMC for aggregates of type `n`.
pub fn aggr_tag<T: Into<InternedString>>(n: T) -> InternedString {
    let n = n.into();
    format!("tag-{n}").into()
}

pub trait NumUtils {
    fn fits_in_bits(&self, width: u64, signed: bool) -> bool;
    fn two_complement(&self, width: u64) -> Self;
}

impl NumUtils for BigInt {
    fn fits_in_bits(&self, width: u64, signed: bool) -> bool {
        self <= &max_int(width, signed) && self >= &min_int(width, signed)
    }

    fn two_complement(&self, width: u64) -> Self {
        assert_eq!(self.sign(), Sign::Minus);
        let max = max_int(width, false);
        assert!(self.abs() < max);
        max - (self.abs() - 1)
    }
}

/// Provides a useful shortcut for making BTreeMaps.
#[macro_export]
macro_rules! btree_map {
    ($($x:expr),*) => {{
        use std::collections::BTreeMap;
        use std::iter::FromIterator;
        (BTreeMap::from_iter(vec![$($x),*]))
    }};
    ($($x:expr,)*) => {{
        use std::collections::BTreeMap;
        use std::iter::FromIterator;
        (BTreeMap::from_iter(vec![$($x),*]))
    }}
}

/// Provides a useful shortcut for making BTreeMaps.
#[macro_export]
macro_rules! linear_map {
    ($($x:expr),*) => {{
        use linear_map::LinearMap;
        use std::iter::FromIterator;
        (LinearMap::from_iter(vec![$($x),*]))
    }};
    ($($x:expr,)*) => {{
        use linear_map::LinearMap;
        use std::iter::FromIterator;
        (LinearMap::from_iter(vec![$($x),*]))
    }}
}

/// Provides a useful shortcut for making BTreeMaps with String keys.
/// Arg: a list of (?, V) tuples, where ? is any type that implements `.to_string()`.
/// Result: BtreeMap<String, V> initilized with the values from the arg list.
#[macro_export]
macro_rules! btree_string_map {
    ($($x:expr),*) => {{
        use std::collections::BTreeMap;
        use std::iter::FromIterator;
        (BTreeMap::from_iter(vec![$($x),*].into_iter().map(|(k,v)|(k.into(),v))))
    }};
    ($($x:expr,)*) => {{
        use std::collections::BTreeMap;
        use std::iter::FromIterator;
        (BTreeMap::from_iter(vec![$($x),*].into_iter().map(|(k,v)|(k.into(),v))))
    }}

}

pub fn max_int(width: u64, signed: bool) -> BigInt {
    let mut bi = BigInt::from(0);
    if signed {
        bi.set_bit(width - 1, true);
    } else {
        bi.set_bit(width, true);
    }
    bi - 1
}

pub fn min_int(width: u64, signed: bool) -> BigInt {
    if signed {
        let max = max_int(width, true);
        -max - 1
    } else {
        BigInt::zero()
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::NumUtils;
    use crate::utils::{max_int, min_int};
    use num::BigInt;

    #[test]
    fn test_fits_in_bits() {
        assert_eq!(BigInt::from(10).fits_in_bits(3, false), false);
        assert_eq!(BigInt::from(10).fits_in_bits(4, false), true);
        assert_eq!(BigInt::from(10).fits_in_bits(5, false), true);
        assert_eq!(BigInt::from(10).fits_in_bits(3, true), false);
        assert_eq!(BigInt::from(10).fits_in_bits(4, true), false);
        assert_eq!(BigInt::from(10).fits_in_bits(5, true), true);

        assert_eq!(BigInt::from(-10).fits_in_bits(3, false), false);
        assert_eq!(BigInt::from(-10).fits_in_bits(4, false), false);
        assert_eq!(BigInt::from(-10).fits_in_bits(5, false), false);
        assert_eq!(BigInt::from(-10).fits_in_bits(3, true), false);
        assert_eq!(BigInt::from(-10).fits_in_bits(4, true), false);
        assert_eq!(BigInt::from(-10).fits_in_bits(5, true), true);
    }

    #[test]
    fn test_twos_complement() {
        assert_eq!(BigInt::from(-10).two_complement(8), BigInt::from(246));
    }

    #[test]
    fn test_max_int() {
        // Unsigned
        assert_eq!(max_int(8, false), BigInt::from(u8::MAX));
        assert_eq!(max_int(16, false), BigInt::from(u16::MAX));
        assert_eq!(max_int(32, false), BigInt::from(u32::MAX));
        assert_eq!(max_int(64, false), BigInt::from(u64::MAX));
        assert_eq!(max_int(128, false), BigInt::from(u128::MAX));

        //Signed
        assert_eq!(max_int(8, true), BigInt::from(i8::MAX));
        assert_eq!(max_int(16, true), BigInt::from(i16::MAX));
        assert_eq!(max_int(32, true), BigInt::from(i32::MAX));
        assert_eq!(max_int(64, true), BigInt::from(i64::MAX));
        assert_eq!(max_int(128, true), BigInt::from(i128::MAX));
    }

    #[test]
    fn test_min_int() {
        // Unsigned
        assert_eq!(min_int(8, false), BigInt::from(u8::MIN));
        assert_eq!(min_int(16, false), BigInt::from(u16::MIN));
        assert_eq!(min_int(32, false), BigInt::from(u32::MIN));
        assert_eq!(min_int(64, false), BigInt::from(u64::MIN));
        assert_eq!(min_int(128, false), BigInt::from(u128::MIN));

        //Signed
        assert_eq!(min_int(8, true), BigInt::from(i8::MIN));
        assert_eq!(min_int(16, true), BigInt::from(i16::MIN));
        assert_eq!(min_int(32, true), BigInt::from(i32::MIN));
        assert_eq!(min_int(64, true), BigInt::from(i64::MIN));
        assert_eq!(min_int(128, true), BigInt::from(i128::MIN));
    }
}
