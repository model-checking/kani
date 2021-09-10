// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --use-abs --abs-type c-ffi
include!{"../../rmc-prelude.rs"}

fn main() {
    let mut h: HashSet<u16> = HashSet::new();

    // TODO: This test should ideally work with nondeterminstic values but for
    // for the moment it does not.
    //
    // let a: u16 = __nondet();
    // let b: u16 = __nondet();
    // let c: u16 = __nondet();
    // __VERIFIER_assume(a != b);
    // __VERIFIER_assume(a != c);
    // __VERIFIER_assume(b != c);

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
