// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --use-abs --abs-type c-ffi
fn main() {
    let mut h: HashSet<u16> = HashSet::new();

    // TODO: This test should ideally work with nondeterminstic values but for
    // for the moment it does not.
    let a: u16 = kani::any();
    let b: u16 = kani::any();
    let c: u16 = kani::any();
    kani::assume(a != b);
    kani::assume(a != c);
    kani::assume(b != c);

    assert!(h.insert(a));
    assert!(h.contains(&a));
    assert!(!h.contains(&b));
    assert!(h.remove(a));
    assert!(!h.contains(&a));
    assert!(!h.contains(&b));
    assert!(h.insert(b));
    assert!(h.contains(&b));
    assert!(h.remove(b));
    assert!(!h.contains(&b));
    assert!(!h.remove(b));
    assert!(h.insert(c));
    assert!(!h.insert(c));
}
