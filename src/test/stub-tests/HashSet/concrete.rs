// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --use-abs --abs-type c-ffi
fn main() {
    let mut h: HashSet<u16> = HashSet::new();

    // TODO: This test should ideally work with nondeterminstic values but for
    // for the moment it does not.
    //
    // let a: u16 = kani::any();
    // let b: u16 = kani::any();
    // let c: u16 = kani::any();
    // kani::assume(a != b);
    // kani::assume(a != c);
    // kani::assume(b != c);

    assert!(h.insert(5));
    assert!(h.contains(&5));
    assert!(!h.contains(&10));
    assert!(h.remove(5));
    assert!(!h.contains(&10));
    assert!(!h.contains(&5));
    assert!(h.insert(0));
    assert!(h.contains(&0));
    assert!(h.remove(0));
    assert!(!h.contains(&0));
    assert!(!h.remove(0));
    assert!(h.insert(6));
    assert!(!h.insert(6));
}
