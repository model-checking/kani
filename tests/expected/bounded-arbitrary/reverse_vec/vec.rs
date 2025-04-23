// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that users can use BoundedArbitrary to perform bounded verification of functions that use Vec.

extern crate kani;

fn reverse_vector<T>(mut input: Vec<T>) -> Vec<T> {
    let mut reversed = vec![];
    for _ in 0..input.len() {
        reversed.push(input.pop().unwrap());
    }
    reversed
}

fn bad_reverse_vector<T: Default, const N: usize>(mut input: Vec<T>) -> Vec<T> {
    let mut reversed = vec![];
    for i in 0..input.len() {
        if i < N {
            reversed.push(input.pop().unwrap());
        } else {
            reversed.push(T::default())
        }
    }
    reversed
}

#[kani::proof]
#[kani::unwind(5)]
fn check_reverse_is_its_own_inverse() {
    let input: Vec<bool> = kani::bounded_any::<_, 4>();
    let double_reversed = reverse_vector(reverse_vector(input.clone()));
    for i in 0..input.len() {
        assert_eq!(input[i], double_reversed[i])
    }
}

#[kani::proof]
#[kani::unwind(17)]
fn check_reverse_is_its_own_inverse_incomplete() {
    let input: Vec<bool> = kani::bounded_any::<_, 16>();
    let double_reversed = bad_reverse_vector::<_, 16>(bad_reverse_vector::<_, 16>(input.clone()));
    for i in 0..input.len() {
        assert_eq!(input[i], double_reversed[i])
    }
}

#[kani::proof]
#[kani::unwind(6)]
fn check_reverse_is_its_own_inverse_should_fail() {
    let input: Vec<bool> = kani::bounded_any::<_, 5>();
    let double_reversed = bad_reverse_vector::<_, 4>(bad_reverse_vector::<_, 4>(input.clone()));
    for i in 0..input.len() {
        assert_eq!(input[i], double_reversed[i])
    }
}
