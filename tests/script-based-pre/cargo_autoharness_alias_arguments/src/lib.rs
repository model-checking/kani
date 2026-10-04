// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test the autoharness `--alias-arguments` option, which models caller-controlled aliasing
// between arguments: a shared reference or raw pointer argument may be the same reference or
// pointer as an earlier argument of the same type, c.f. the `AnyAlias` model. Without the
// option, each such argument refers to its own allocation.
// The "TEST NOTE" comments give the expected result per function, without and with the option.

use core::cell::Cell;

// TEST NOTE: without: PASS (separate allocations); with: FAIL (`b` may be `a`).
pub fn refs_distinct(a: &u8, b: &u8) {
    assert!(!core::ptr::eq(a, b));
}

// TEST NOTE: without: PASS; with: FAIL. The regions of the two references are not compared,
// so distinct lifetime parameters do not prevent aliasing.
pub fn refs_distinct_lifetimes<'a, 'b>(a: &'a u8, b: &'b u8) {
    assert!(!core::ptr::eq(a, b));
}

// TEST NOTE: without: PASS; with: FAIL (`b` may be the same slice as `a`).
pub fn slices_distinct(a: &[u8], b: &[u8]) {
    assert!(!core::ptr::eq(a, b));
}

pub struct OnlyDerivable {
    pub v: u8,
}

// TEST NOTE: without: PASS; with: FAIL. As `slices_distinct`, for slices generated with a
// bounded length from harness-local storage, which requires `--bounded-arguments`: the
// compiler derives the element type's Arbitrary implementation.
pub fn bounded_slices_distinct(a: &[OnlyDerivable], b: &[OnlyDerivable]) {
    assert!(!core::ptr::eq(a, b));
}

// TEST NOTE: without: PASS; with: FAIL, since writing through `a` changes `b` if they alias.
pub fn cell_no_alias(a: &Cell<u8>, b: &Cell<u8>) {
    let old = b.get();
    a.set(old.wrapping_add(1));
    assert!(b.get() == old);
}

// TEST NOTE: without: PASS; with: FAIL, since all three arguments may be the same reference.
pub fn three_not_all_same(a: &u8, b: &u8, c: &u8) {
    assert!(!(core::ptr::eq(a, b) && core::ptr::eq(b, c)));
}

// TEST NOTE: without: PASS; with: FAIL, since the third argument may alias the second one
// without either aliasing the first.
pub fn last_two_distinct(a: &u8, b: &u8, c: &u8) {
    assert!(core::ptr::eq(a, b) || !core::ptr::eq(b, c));
}

// TEST NOTE: without: PASS; with: FAIL, since the third argument may alias the first one
// while the second one does not.
pub fn first_and_last_distinct(a: &u8, b: &u8, c: &u8) {
    assert!(core::ptr::eq(a, b) || !core::ptr::eq(a, c));
}

// TEST NOTE: without: PASS, since two non-null pointers point into (or one past the end of)
// separate allocations; with: FAIL (`q` may be `p`). Null is excluded because both pointers
// may be null in either mode.
pub fn const_ptrs_distinct(p: *const u8, q: *const u8) {
    if !p.is_null() && !q.is_null() {
        assert!(p != q);
    }
}

// TEST NOTE: without: PASS; with: FAIL. As `const_ptrs_distinct`, for mutable raw pointers,
// which (unlike `&mut T`) may alias.
pub fn mut_ptrs_distinct(p: *mut u8, q: *mut u8) {
    if !p.is_null() && !q.is_null() {
        assert!(p != q);
    }
}

// TEST NOTE: PASS in both modes: a `&mut T` is exclusive, so it never aliases.
pub fn muts_distinct(a: &mut u8, b: &mut u8) {
    assert!(!core::ptr::eq(a, b));
}

// TEST NOTE: PASS in both modes: a `&mut T` aliases no shared reference either.
pub fn ref_and_mut_distinct(a: &u8, b: &mut u8) {
    assert!(!core::ptr::eq(a, b));
}

// TEST NOTE: PASS in both modes: arguments of different types do not alias.
pub fn different_types_distinct(a: &u8, b: &u16) {
    assert!(a as *const u8 != b as *const u16 as *const u8);
}

// TEST NOTE: PASS in both modes: a reference and a raw pointer do not alias, even if they
// have the same pointee type.
pub fn ref_and_ptr_distinct(a: &u8, p: *const u8) {
    assert!(a as *const u8 != p);
}

// TEST NOTE: PASS in both modes: raw pointers of different mutability do not alias.
pub fn different_ptr_mutability_distinct(p: *const u8, q: *mut u8) {
    if !p.is_null() && !q.is_null() {
        assert!(p != q as *const u8);
    }
}

// TEST NOTE: PASS in both modes: the function is correct whether or not its arguments alias.
pub fn alias_safe(a: &u8, b: &u8) -> u8 {
    let max = if *a >= *b { *a } else { *b };
    assert!(!core::ptr::eq(a, b) || max == *a);
    max
}

// TEST NOTE: without: PASS; with: FAIL. Only the regions of the two arguments differ, here
// in the pointee type, which does not prevent aliasing either.
pub fn nested_refs_distinct<'a, 'b>(a: &&'a u8, b: &&'b u8) {
    assert!(!core::ptr::eq(a, b));
}

// TEST NOTE: PASS in both modes: the precondition excludes aliasing arguments, so the
// contract harness does not consider them. (The `modifies` clause is what allows the write
// through `a`; it is needed in either mode.)
#[kani::requires(!core::ptr::eq(a, b))]
#[kani::modifies(a)]
pub fn contract_excludes_alias(a: &Cell<u8>, b: &Cell<u8>) {
    let old = b.get();
    a.set(old.wrapping_add(1));
    assert!(b.get() == old);
}

// TEST NOTE: without: PASS; with: FAIL, since the precondition does not exclude aliasing
// arguments, for which the assertion does not hold.
#[kani::requires(b.get() < u8::MAX)]
#[kani::modifies(a)]
pub fn contract_allows_alias(a: &Cell<u8>, b: &Cell<u8>) {
    let old = b.get();
    a.set(old + 1);
    assert!(b.get() == old);
}
