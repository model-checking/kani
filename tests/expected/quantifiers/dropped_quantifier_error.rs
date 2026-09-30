// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zquantifiers

//! Test that Kani fails with a sound-analysis error when the solver backend
//! drops a quantifier it cannot encode. CBMC's SAT-based backends only support
//! quantifiers with constant bounds; a quantifier with a symbolic bound is
//! replaced by an unconstrained value (CBMC prints only a low-visibility
//! "warning: ignoring forall"). That silently vacuates `kani::assume`s: without
//! intervention this harness would report SUCCESSFUL only because the final
//! assertion does not depend on the (unenforced) assumption -- an unsound false
//! negative. Kani must instead surface the error and force a failure.

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
