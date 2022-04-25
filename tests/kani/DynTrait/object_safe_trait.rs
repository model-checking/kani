// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Example from https://doc.rust-lang.org/reference/items/traits.html#object-safety
//
// The foo, param and typed functions of the trait will not appear in the vtable
// of obj

trait NonDispatchable {
    // Non-methods cannot be dispatched.
    fn foo()
    where
        Self: Sized,
    {
    }
    // Self type isn't known until runtime.
    fn returns(&self) -> Self
    where
        Self: Sized;
    // `other` may be a different concrete type of the receiver.
    fn param(&self, other: Self)
    where
        Self: Sized,
    {
    }
    // Generics are not compatible with vtables.
    fn typed<T>(&self, x: T)
    where
        Self: Sized,
    {
    }
}

struct S;
impl NonDispatchable for S {
    fn returns(&self) -> Self
    where
        Self: Sized,
    {
        S
    }
}

#[kani::proof]
fn main() {
    let s = S {};
    S::foo();
    let t = s.returns();
    s.param(t);
    s.typed(0u32);
    s.typed(true);

    let _obj: Box<dyn NonDispatchable> = Box::new(s);
}
