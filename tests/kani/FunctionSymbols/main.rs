// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test code generation for FnDef items

#[kani::proof]
fn test_reify_fn_pointer() {
    assert!(poly::<usize> as fn() == poly::<usize> as fn());
    assert!(poly::<isize> as fn() != poly::<usize> as fn());
}

fn poly<T>() {}

#[kani::proof]
fn test_fn_pointer_call() {
    let x: bool = kani::any();
    assert_eq!(id(x), x);
    assert_eq!((id::<bool> as fn(bool) -> bool)(x), x);
}

fn id<T>(x: T) -> T {
    x
}

struct Wrapper<T> {
    inner: T,
}

#[kani::proof]
fn test_fn_wrapper() {
    let w = Wrapper { inner: id::<bool> };
    assert!(w.inner as fn(bool) -> bool == id::<bool> as fn(bool) -> bool);
    let x: bool = kani::any();
    assert_eq!((w.inner)(x), x);
}
