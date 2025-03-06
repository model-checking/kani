// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --default-unwind 17
//
// Check that users can implement Arbitrary to a simple data struct with Vec<>.
extern crate kani;
use kani::BoundedAny;

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
fn check_reverse_is_its_own_inverse() {
    let input: BoundedAny<Vec<bool>, 4> = kani::any();
    let double_reversed = reverse_vector(reverse_vector(input.clone().into_inner()));
    for i in 0..input.len() {
        assert!(input[i] == double_reversed[i])
    }
}

#[kani::proof]
fn check_reverse_is_its_own_inverse_incomplete() {
    let input: BoundedAny<Vec<bool>, 16> = kani::any();
    let double_reversed =
        bad_reverse_vector::<_, 16>(bad_reverse_vector::<_, 16>(input.clone().into_inner()));
    for i in 0..input.len() {
        assert!(input[i] == double_reversed[i])
    }
}

#[kani::proof]
fn check_reverse_is_its_own_inverse_should_fail() {
    let input: BoundedAny<Vec<bool>, 5> = kani::any();
    let double_reversed =
        bad_reverse_vector::<_, 4>(bad_reverse_vector::<_, 4>(input.clone().into_inner()));
    for i in 0..input.len() {
        kani::cover!(input[i] == double_reversed[i], "This may be equal")
    }
}
