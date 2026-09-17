// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that the autoharness subcommand supports functions with raw pointer arguments.
// The generated harness produces pointers in a nondeterministic allocation state:
// null, out of bounds, or valid (pointing to a nondeterministic value that lives for the
// entire harness), c.f. the `AnyPtr` model.
// The "TEST NOTE" comments below explain the expected verification result for each function.

// TEST NOTE: a struct with a raw-pointer field, defined and used BEFORE the plain
// raw-pointer functions below. `has_ptr_field` must be skipped (structs with pointer fields
// cannot derive Arbitrary), and, crucially, processing it must not poison the eligibility
// cache for `*const i64` in argument position (c.f. `ptr_arg_after_struct` at the bottom).
pub struct HasPtrField {
    p: *const i64,
}

pub fn has_ptr_field(h: HasPtrField) -> usize {
    h.p as usize
}

// TEST NOTE: should FAIL, since the pointer may be null or out of bounds.
pub fn unchecked_deref(p: *const i32) -> i32 {
    unsafe { *p }
}

// TEST NOTE: should FAIL, since a null check is not sufficient: the pointer may still be
// out of bounds.
pub fn null_check_only(p: *const i32) -> Option<i32> {
    if p.is_null() { None } else { Some(unsafe { *p }) }
}

// TEST NOTE: should PASS, since it only manipulates the pointer's address.
pub fn ptr_to_addr(p: *const u8) -> usize {
    p as usize
}

// TEST NOTE: should PASS: the contract harness assumes the precondition, which excludes the
// null and out-of-bounds states, and the remaining valid state points to allocated memory
// that Kani's memory predicates can reason about.
#[kani::requires(kani::mem::can_dereference(p))]
pub fn contract_deref(p: *const i32) -> i32 {
    unsafe { *p }
}

// TEST NOTE: should PASS, like `contract_deref` but writing through a mutable pointer.
#[kani::requires(kani::mem::can_write(p))]
#[kani::modifies(p)]
#[kani::ensures(|_| unsafe { *p } == 7)]
pub fn contract_write(p: *mut i32) {
    unsafe { *p = 7 };
}

// TEST NOTE: should PASS: nested pointers are supported; this function only manipulates the
// address.
pub fn nested_ptr_addr(p: *mut *mut u32) -> usize {
    p as usize
}

// TEST NOTE: should FAIL: each level of the nested pointer may be null or out of bounds.
pub fn nested_ptr_deref(p: *mut *mut u32) -> u32 {
    unsafe { **p }
}

// TEST NOTE: should PASS, and the cover check must be SATISFIED, i.e., the valid pointer
// state must be reachable and readable.
pub fn valid_state_reachable(p: *const i32) {
    if kani::mem::can_dereference(p) {
        let v = unsafe { *p };
        kani::cover!(v == 42, "read 42 through a valid pointer");
    }
}

// TEST NOTE: should PASS: `*const i64` in argument position is supported even though
// `has_ptr_field` above (processed first) cached the verdict that `*const i64` does not
// implement Arbitrary as a struct field.
pub fn ptr_arg_after_struct(p: *const i64) -> usize {
    p as usize
}

// TEST NOTE: must be skipped, even though `*const i32` was accepted in argument position by
// the functions above: a struct with a raw-pointer field still cannot derive Arbitrary.
pub struct HasPtrFieldLast {
    p: *const i32,
}

pub fn has_ptr_field_last(h: HasPtrFieldLast) -> usize {
    h.p as usize
}
