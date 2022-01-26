// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Test using the unit type
///   - As a void function return
///   - As a variable type in a struct or union

struct S {
    f: (),
}

#[repr(C)]
union U {
    f: (),
    g: i32,
}

fn ret_unit() {
    ()
}

fn main() {
    assert!(() == ());
    let u = ret_unit();
    assert!(u == ());
    let s = S { f: () };
    assert!(s.f == ());
    let u = U { f: () };
    unsafe {
        assert!(u.f == ());
    }
    // TODO: determine whether we can say anything sensible about u.g
    // i.e., assert!(u.g == ???)

    // TODO: determine whether the following is defined
    let mut u = U { f: () };
    unsafe {
        u.g = 42;
        u.f = ();
        assert!(u.g == 42);
    }
}
