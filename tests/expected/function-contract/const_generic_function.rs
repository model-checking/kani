// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check that Kani contract can be applied to a constant generic function.
//! <https://github.com/model-checking/kani/issues/3667>

struct Foo<T> {
    ptr: *const T,
}

impl<T: Sized> Foo<T> {
    #[kani::requires(true)]
    pub const unsafe fn bar(self, v: T)
    where
        T: Sized,
    {
        unsafe { core::ptr::write(self.ptr as *mut T, v) };
    }
}

#[kani::proof_for_contract(Foo::bar)]
fn check_const_generic_function() {
    let x: u8 = kani::any();
    let foo: Foo<u8> = Foo { ptr: &x };
    unsafe { foo.bar(kani::any::<u8>()) };
}
