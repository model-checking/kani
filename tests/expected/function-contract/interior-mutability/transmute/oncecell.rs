// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// The objective of this test is to check the modification of an OnceCell used as interior mutability in an immutable struct

/// ---------------------------------------------------
///        Abstraction Breaking Functionality
/// ---------------------------------------------------
use std::cell::OnceCell;
use std::mem::transmute;

/// This exposes the underlying representation so that it can be added into a modifies clause within kani
trait Exposeable<T: ?Sized> {
    unsafe fn expose(&self) -> &T;
}

/// While this is not explicitly labeled as safe in the Rust documentation, it works due to OnceCell having a single field in its struct definition
impl<T> Exposeable<Option<T>> for OnceCell<T> {
    unsafe fn expose(&self) -> &Option<T> {
        transmute(self)
    }
}

/// ---------------------------------------------------
///                      Test Case
/// ---------------------------------------------------

/// This struct is contains OnceCell which can be mutated
struct InteriorMutability {
    x: OnceCell<u32>,
}

/// contracts need to access im.x internal data through the unsafe function im.x.expose()
#[kani::requires(unsafe{im.x.expose()}.is_none())]
#[kani::modifies(im.x.expose())]
#[kani::ensures(|_| unsafe{im.x.expose()}.is_some())]
fn modify(im: &InteriorMutability) {
    // method for setting value in OnceCell without breaking encapsulation
    im.x.set(5).expect("")
}

#[kani::proof_for_contract(modify)]
fn harness_for_modify() {
    let im: InteriorMutability = InteriorMutability { x: OnceCell::new() };
    modify(&im)
}
