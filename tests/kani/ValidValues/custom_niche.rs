// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Check that Kani can identify UB when using niche attribute for a custom operation.
#![feature(rustc_attrs)]

use std::mem;
use std::mem::size_of;

/// A possible implementation for a system of rating that defines niche.
/// A Rating represents the number of stars of a given product (1..=5).
#[rustc_layout_scalar_valid_range_start(1)]
#[rustc_layout_scalar_valid_range_end(5)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rating {
    stars: u8,
}

impl kani::Arbitrary for Rating {
    fn any() -> Self {
        let stars = kani::any_where(|s: &u8| *s >= 1 && *s <= 5);
        unsafe { Rating { stars } }
    }
}

impl Rating {
    /// Buggy version of new. Note that this still creates an invalid Rating.
    pub fn new(value: u8) -> Option<Rating> {
        (value > 0 && value <= 5).then_some(unsafe { Rating { stars: value } })
    }

    pub unsafe fn new_unchecked(stars: u8) -> Rating {
        Rating { stars }
    }
}

#[kani::proof]
#[kani::should_panic]
pub fn check_new_with_ub() {
    assert_eq!(Rating::new(10), None);
}
#[test]
fn kani_concrete_playback_check_new_with_ub_16826550562391574683() {
    let concrete_vals: Vec<Vec<u8>> = vec![];
    kani::concrete_playback_run(concrete_vals, check_new_with_ub);
}

#[kani::proof]
#[kani::should_panic]
pub fn check_unchecked_new_ub() {
    let val = kani::any();
    assert_eq!(unsafe { Rating::new_unchecked(val).stars }, val);
}
#[test]
fn kani_concrete_playback_check_unchecked_new_ub_1626477376985561705() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 255
        vec![255],
    ];
    kani::concrete_playback_run(concrete_vals, check_unchecked_new_ub);
}

#[kani::proof]
#[kani::should_panic]
pub fn check_new_with_ub_limits() {
    let stars = kani::any_where(|s: &u8| *s == 0 || *s > 5);
    let _ = Rating::new(stars);
    unreachable!("Call to new should always fail");
}
#[test]
fn kani_concrete_playback_check_new_with_ub_limits_5138089996830704245() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 0
        vec![0],
    ];
    kani::concrete_playback_run(concrete_vals, check_new_with_ub_limits);
}

#[kani::proof]
#[kani::should_panic]
pub fn check_invalid_dereference() {
    let any: u8 = kani::any();
    let _rating: Rating = unsafe { *(&any as *const _ as *const _) };
}
#[test]
fn kani_concrete_playback_check_invalid_dereference_17514599809884320235() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 255
        vec![255],
    ];
    kani::concrete_playback_run(concrete_vals, check_invalid_dereference);
}

#[kani::proof]
#[kani::should_panic]
pub fn check_invalid_transmute() {
    let any: u8 = kani::any();
    let _rating: Rating = unsafe { mem::transmute(any) };
}
#[test]
fn kani_concrete_playback_check_invalid_transmute_3709630971955203287() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 255
        vec![255],
    ];
    kani::concrete_playback_run(concrete_vals, check_invalid_transmute);
}

#[kani::proof]
#[kani::should_panic]
pub fn check_invalid_transmute_copy() {
    let any: u8 = kani::any();
    let _rating: Rating = unsafe { mem::transmute_copy(&any) };
}
#[test]
fn kani_concrete_playback_check_invalid_transmute_copy_442881509506342923() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 255
        vec![255],
    ];
    kani::concrete_playback_run(concrete_vals, check_invalid_transmute_copy);
}

#[kani::proof]
#[kani::should_panic]
pub fn check_invalid_increment() {
    let mut orig: Rating = kani::any();
    unsafe { orig.stars += 1 };
}
#[test]
fn kani_concrete_playback_check_invalid_increment_7950917765053634540() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 5
        vec![5],
    ];
    kani::concrete_playback_run(concrete_vals, check_invalid_increment);
}

#[kani::proof]
pub fn check_valid_increment() {
    let mut orig: Rating = kani::any();
    kani::assume(orig.stars < 5);
    unsafe { orig.stars += 1 };
}

/// Check that the compiler relies on valid value range of Rating to implement niche optimization.
#[kani::proof]
pub fn check_niche() {
    assert_eq!(size_of::<Rating>(), size_of::<Option<Rating>>());
    assert_eq!(size_of::<Rating>(), size_of::<Option<Option<Rating>>>());
}
