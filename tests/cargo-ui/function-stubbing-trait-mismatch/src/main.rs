// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This tests that we catch trait mismatches between the stub and the original
//! function/method. In particular, this tests the case when the program is
//! treated as an executable (i.e., the `--crate-type bin` rustc option).

fn foo<T>(_x: T) -> bool {
    false
}

trait DoIt {
    fn do_it(&self) -> bool;
}

fn bar<T: DoIt + std::cmp::PartialEq<i32>>(x: T) -> bool {
    x.do_it() && x == 42
}

#[kani::proof]
#[kani::stub(foo, bar)]
fn main() {
    assert!(foo("hello"));
}
