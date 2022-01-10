// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --use-abs --abs-type c-ffi
fn main() {
    let mut h: HashSet<u16> = HashSet::new();

    // TODO: This test should ideally work with nondeterminstic values but for
    // for the moment it does not.
    let a: u16 = rmc::any();
    let b: u16 = rmc::any();
    let c: u16 = rmc::any();
    rmc::assume(a != b);
    rmc::assume(a != c);
    rmc::assume(b != c);

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
