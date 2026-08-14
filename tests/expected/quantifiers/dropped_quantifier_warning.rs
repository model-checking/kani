// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zquantifiers

//! Test that Kani prominently warns when the solver backend drops a quantifier
//! it cannot encode. CBMC's SAT-based backends only support quantifiers with
//! constant bounds; a quantifier with a symbolic bound is replaced by an
//! unconstrained value (CBMC prints only a low-visibility "warning: ignoring
//! forall"). This silently vacuates `kani::assume`s: this harness SUCCEEDS,
//! but only because the final assertion does not depend on the (unenforced)
//! assumption -- which is exactly why the warning must be prominent.

extern crate kani;

#[kani::proof]
fn vacuous_assume_warns() {
    let len: usize = kani::any();
    kani::assume(len >= 1 && len <= 100);
    let layout = std::alloc::Layout::array::<u8>(len).unwrap();
    let p = unsafe { std::alloc::alloc(layout) };
    kani::assume(!p.is_null());
    unsafe {
        kani::assume(kani::forall!(|i in (0, len)| *p.wrapping_add(i) < 60));
    }
    assert!(len <= 100);
}
