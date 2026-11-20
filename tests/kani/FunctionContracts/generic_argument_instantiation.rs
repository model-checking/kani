// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// `proof_for_contract` on a target whose type carries generic arguments — the remaining
// generic-parameter case from https://github.com/model-checking/kani/issues/1997.
// `resolve_ty` now instantiates a path type's generic arguments before resolution, so
// these all resolve and verify:
//   * a generic *argument* that is itself generic (`Generic<Wrap<u8>>`),
//   * a generic *self* type with an associated-type return and a `where`-bounded method,
//   * a concrete DST self type with a generic argument (the `<CStr as Index<RangeFrom>>` shape),
//   * a generic *impl* (`impl<T> Get for W<T>`) queried at a concrete instantiation,
//   * a type with a lifetime parameter beside a type parameter (`Borrowed<'a, T>`) — lifetime erased.
// `check_primitive_arg` guards that the already-working concrete-argument case is unchanged.

struct S(u32);
struct Wrap<T>(T);

trait Generic<T> {
    fn generic(&self) -> u32;
}

impl Generic<u8> for S {
    #[kani::requires(self.0 < 100)]
    #[kani::ensures(|r| *r == self.0)]
    fn generic(&self) -> u32 {
        self.0
    }
}

// Distinct body from the `Generic<u8>` impl above, so the harness only verifies
// if resolution lands on *this* impl rather than the sibling.
impl Generic<Wrap<u8>> for S {
    #[kani::requires(self.0 < 100)]
    #[kani::ensures(|r| *r == self.0 + 1)]
    fn generic(&self) -> u32 {
        self.0 + 1
    }
}

trait Access {}

struct W<T>(T);

impl Access for W<u8> {}

trait Indexed {
    type Item;
    unsafe fn get_unchecked(&self, i: usize) -> Self::Item
    where
        Self: Access;
}

impl Indexed for W<u8> {
    type Item = u8;
    #[kani::requires(i < 1)]
    #[kani::ensures(|r| *r == 0)]
    unsafe fn get_unchecked(&self, i: usize) -> u8
    where
        Self: Access,
    {
        let _ = i;
        0
    }
}

trait Get {
    fn get(&self) -> u32;
}

// A generic impl: the query `<W<u8> as Get>::get` supplies the concrete instantiation.
impl<T> Get for W<T> {
    #[kani::requires(true)]
    #[kani::ensures(|r| *r == 7)]
    fn get(&self) -> u32 {
        7
    }
}

#[repr(transparent)]
struct Dst([u8]);

trait Peek<T> {
    fn peek(&self) -> u32;
}

impl Peek<Wrap<u8>> for Dst {
    #[kani::requires(true)]
    #[kani::ensures(|r| *r as usize == self.0.len())]
    fn peek(&self) -> u32 {
        self.0.len() as u32
    }
}

struct Borrowed<'a, T>(&'a T);

trait Held {
    fn held(&self) -> u32;
}

impl<'a, T> Held for Borrowed<'a, T> {
    #[kani::requires(true)]
    #[kani::ensures(|r| *r == 0)]
    fn held(&self) -> u32 {
        0
    }
}

#[cfg(kani)]
mod verify {
    use super::*;

    // Concrete (primitive) argument — resolved before this change; must still resolve.
    #[kani::proof_for_contract(<S as Generic<u8>>::generic)]
    fn check_primitive_arg() {
        let s = S(kani::any());
        let _ = Generic::<u8>::generic(&s);
    }

    // Generic argument that is itself generic — now resolves.
    #[kani::proof_for_contract(<S as Generic<Wrap<u8>>>::generic)]
    fn check_generic_arg() {
        let s = S(kani::any());
        let _ = Generic::<Wrap<u8>>::generic(&s);
    }

    // Generic self type + associated-type return + `where`-bounded method — now resolves.
    #[kani::proof_for_contract(<W<u8> as Indexed>::get_unchecked)]
    fn check_generic_self() {
        let w = W(0u8);
        let _ = unsafe { <W<u8> as Indexed>::get_unchecked(&w, kani::any()) };
    }

    // Concrete DST self with a generic argument — now resolves.
    #[kani::proof_for_contract(<Dst as Peek<Wrap<u8>>>::peek)]
    fn check_dst_self() {
        let bytes = [1u8, 2, 3];
        let dst: &Dst = unsafe { &*(&bytes[..] as *const [u8] as *const Dst) };
        let _ = dst.peek();
    }

    // Generic impl queried at a concrete instantiation — now resolves.
    #[kani::proof_for_contract(<W<u8> as Get>::get)]
    fn check_generic_impl() {
        let w = W(0u8);
        let _ = w.get();
    }

    // Lifetime parameter beside a type parameter — lifetime erased, type substituted.
    #[kani::proof_for_contract(<Borrowed<'static, u8> as Held>::held)]
    fn check_lifetime_param() {
        let x = 0u8;
        let b = Borrowed(&x);
        let _ = b.held();
    }
}
