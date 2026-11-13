// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixture workspace for the Crate Verifiability Map tests: loops, recursion (direct and mutual),
//! generics, trait dispatch, macros, features, contracts, and harnesses that pass and fail.

pub mod math;
pub mod shapes;

/// Generates a function: invisible to the source scan (a known limitation).
macro_rules! make_const_fn {
    ($name:ident, $value:expr) => {
        pub fn $name() -> u32 {
            $value
        }
    };
}
make_const_fn!(seven, 7);

/// A loop.
pub fn sum_to(n: u32) -> u32 {
    let mut s = 0;
    let mut i = 0;
    while i < n {
        s += i;
        i += 1;
    }
    s
}

/// Direct recursion.
pub fn fact(n: u32) -> u32 {
    if n == 0 { 1 } else { n * fact(n - 1) }
}

/// Mutual recursion.
pub fn is_even(n: u32) -> bool {
    if n == 0 { true } else { is_odd(n - 1) }
}

pub fn is_odd(n: u32) -> bool {
    if n == 0 { false } else { is_even(n - 1) }
}

#[cfg(feature = "fast")]
pub fn fast_path(x: u32) -> u32 {
    x.wrapping_mul(3)
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    #[kani::unwind(5)]
    fn check_sum_to() {
        let n: u32 = kani::any();
        kani::assume(n < 4);
        assert!(sum_to(n) <= 6);
        kani::cover!(n == 3);
    }

    /// Fails: `fact` overflows for large `n`.
    #[kani::proof]
    #[kani::unwind(15)]
    fn check_fact() {
        let n: u32 = kani::any();
        kani::assume(n < 14);
        assert!(fact(n) >= 1);
    }
}
