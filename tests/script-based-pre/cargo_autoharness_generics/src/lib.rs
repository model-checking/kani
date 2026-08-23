// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports generic functions by instantiating their type
// parameters with a concrete type: the first candidate (starting from `i32`) that satisfies all
// of the function's trait bounds. Lifetime parameters are erased; functions with const generic
// parameters or with bounds that no candidate satisfies are skipped.
// The "TEST NOTE" comments below explain the expected result for each function.

// TEST NOTE: verified as `identity::<i32>`.
pub fn identity<T>(x: T) -> T {
    x
}

// TEST NOTE: verified as `max3::<i32>`; primitives satisfy the bounds.
pub fn max3<T: Copy + PartialOrd>(a: T, b: T, c: T) -> T {
    let mut m = a;
    if b > m {
        m = b;
    }
    if c > m {
        m = c;
    }
    m
}

// TEST NOTE: verified as `buggy_add::<i32>` and FAILS, since the addition can overflow.
// This demonstrates that instantiating a generic function can find real bugs.
pub fn buggy_add<T: Copy + core::ops::Add<Output = T>>(a: T, b: T) -> T {
    a + b
}

// TEST NOTE: verified as `pair::<i32, i32>`; multiple type parameters are supported.
pub fn pair<T: Copy, U: Default>(x: T, _y: U) -> (T, U) {
    (x, U::default())
}

// TEST NOTE: verified as `first::<i32>`; lifetime parameters are erased.
pub fn first<'a, T: Copy>(x: &'a T) -> T {
    *x
}

// TEST NOTE: verified as `takes_impl::<u32>`: `i32` does not satisfy `Into<u64>`,
// so the next candidate that does (`u32`) is chosen.
pub fn takes_impl(x: impl Into<u64> + Copy) -> u64 {
    x.into()
}

// TEST NOTE: skipped (Generic Function), since no candidate type implements `Exotic`.
pub trait Exotic {
    fn exotic(&self) -> u8;
}
pub fn needs_exotic<T: Exotic>(x: T) -> u8 {
    x.exotic()
}

// TEST NOTE: verified as `with_const::<2>`; usize const generic parameters are instantiated
// with the value 2.
pub fn with_const<const N: usize>(x: [u8; N]) -> usize {
    x.len() + N
}

// TEST NOTE: skipped (Generic Function), since non-usize const generic parameters are not
// supported yet.
pub fn with_bool_const<const B: bool>(x: u8) -> u8 {
    if B { x } else { 0 }
}

// TEST NOTE: verified as `Wrapper::<i32>::get`; generic parameters of the impl block are
// instantiated too.
pub struct Wrapper<T> {
    val: T,
}

impl<T: Copy> Wrapper<T> {
    pub fn get(&self) -> T {
        self.val
    }
}

// TEST NOTE: verified as `contracted::<u32>` with a contract harness; the contract is checked
// for the chosen instantiation.
#[kani::requires(x < 1000)]
#[kani::ensures(|r| *r >= x)]
pub fn contracted<T: Copy + Into<u64>>(_marker: T, x: u64) -> u64 {
    x + 1
}
