// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check the unsized coercion of a struct whose unsized tail is an associated type projection,
//! e.g. `Wrap<Array<N>>` (tail `[T; N]`) to `Wrap<Slice>` (tail `[T]`).
//! Kani used to crash during codegen because it did not normalize the projection when computing
//! the pointer metadata, so it generated a thin pointer where a fat pointer was required.
//! See <https://github.com/model-checking/kani/issues/4812>.

trait Storage {
    type Buffer: ?Sized;
}

struct Array<T, const N: usize>(core::marker::PhantomData<T>);
impl<T, const N: usize> Storage for Array<T, N> {
    type Buffer = [T; N];
}

struct Slice<T>(core::marker::PhantomData<T>);
impl<T> Storage for Slice<T> {
    type Buffer = [T];
}

struct Wrap<S: Storage> {
    _b: S::Buffer,
}

fn coerce<T, const N: usize>(this: &Wrap<Array<T, N>>) -> &Wrap<Slice<T>> {
    coerce_again(this)
}

fn coerce_again<T, const N: usize>(this: &Wrap<Array<T, N>>) -> &Wrap<Slice<T>> {
    this
}

#[kani::proof]
fn check_assoc_type_tail_coercion() {
    let val: u8 = kani::any();
    let inner: Wrap<Array<u8, 2>> = Wrap { _b: [val; 2] };
    let coerced: &Wrap<Slice<u8>> = coerce(&inner);
    assert_eq!(coerced._b.len(), 2);
    assert_eq!(coerced._b[0], val);
    assert_eq!(coerced._b[1], val);
}

#[kani::proof]
fn check_assoc_type_tail_coercion_zst() {
    // Same as above, but with a zero-sized element type (the original reproducer).
    let inner: Wrap<Array<(), 1>> = Wrap { _b: [(); 1] };
    let coerced: &Wrap<Slice<()>> = coerce(&inner);
    assert_eq!(coerced._b.len(), 1);
}
