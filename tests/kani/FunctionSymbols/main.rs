// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// examples discovered while debugging issue https://github.com/model-checking/kani/issues/1243

#[kani::proof]
fn example1() {
    f(h);
}

fn f(g: impl Fn() -> ()) {
    move || g();
}

fn h() {}

#[kani::proof]
fn example2() {
    std::iter::empty().try_fold(0, map_try_fold(&mut |x: usize| x, usize::checked_add));
}

fn map_try_fold<'a, T, B, Acc, R>(
    f: &'a mut impl FnMut(T) -> B,
    mut g: impl FnMut(Acc, B) -> R + 'a,
) -> impl FnMut(Acc, T) -> R + 'a {
    move |acc, elt| g(acc, f(elt))
}

#[kani::proof]
fn example3() {
    Vec::<String>::new().join("");
}

// other FnDef tests:

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
