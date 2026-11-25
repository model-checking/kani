// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts

//! `panics_if(cond)` requires a panic whenever `cond` holds. Alone, it also permits panics only
//! when `cond` holds; with a `panics_only_if(c)` clause, `cond` must imply `c`.

/// Panics if and only if `index > len`.
#[kani::panics_if(index > len)]
#[kani::ensures(|result: &usize| *result == index)]
fn check_index(len: usize, index: usize) -> usize {
    assert!(index <= len, "index out of range");
    index
}

#[kani::proof_for_contract(check_index)]
fn check_exact() {
    check_index(kani::any(), kani::any());
}

/// Returns for some inputs where it must panic.
#[kani::panics_if(x > 10)]
fn too_lenient(x: u8) -> u8 {
    if x > 20 {
        panic!("big");
    }
    x
}

#[kani::proof_for_contract(too_lenient)]
fn check_too_lenient() {
    too_lenient(kani::any());
}

/// Panics for some inputs where it must not.
#[kani::panics_if(x > 20)]
fn too_strict(x: u8) -> u8 {
    if x > 10 {
        panic!("big");
    }
    x
}

#[kani::proof_for_contract(too_strict)]
fn check_too_strict() {
    too_strict(kani::any());
}

/// Must panic if `x > 20`, may panic if `x > 10`.
#[kani::panics_only_if(x > 10)]
#[kani::panics_if(x > 20)]
fn may_or_must(x: u8, flip: bool) -> u8 {
    if x > 20 || (x > 10 && flip) {
        panic!("big");
    }
    x
}

#[kani::proof_for_contract(may_or_must)]
fn check_both() {
    may_or_must(kani::any(), kani::any());
}

/// Inconsistent: must panic if `x > 5`, but may only panic if `x > 10`.
#[kani::panics_only_if(x > 10)]
#[kani::panics_if(x > 5)]
fn inconsistent(x: u8) -> u8 {
    if x > 5 {
        panic!("big");
    }
    x
}

#[kani::proof_for_contract(inconsistent)]
fn check_inconsistent() {
    inconsistent(kani::any());
}
