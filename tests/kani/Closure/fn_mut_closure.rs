// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can pass a FnMut closure to a stand alone
// function definition.

fn each<T, F>(x: &[T], mut f: F)
where
    F: FnMut(&T),
{
    for val in x {
        f(val)
    }
}

#[kani::proof]
#[kani::unwind(6)]
fn main() {
    let mut sum = 0_usize;
    let elems = [1_usize, 2, 3, 4, 5];
    each(&elems, |val| sum += *val);
    assert_eq!(sum, 15);
}
