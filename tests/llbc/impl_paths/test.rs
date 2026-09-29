// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend names methods the way Charon does, with the `impl`
//! block as a path element: `{Wrapper<T>}` for an inherent impl (bound by the impl's generics)
//! and `{impl Tr<U> for Wrapper<T>}` for a trait impl, which is declared. The method's own name
//! used to carry the implementing type as a suffix (`getWrapper<T>`).

struct Wrapper<T> {
    v: T,
}

impl<T: Copy> Wrapper<T> {
    fn get(&self) -> T {
        self.v
    }
}

trait Tr<U> {
    fn m(&self, u: U) -> U;
}

impl<T: Copy, U> Tr<U> for Wrapper<T> {
    fn m(&self, u: U) -> U {
        u
    }
}

impl<'a> Tr<u8> for &'a u32 {
    fn m(&self, u: u8) -> u8 {
        u
    }
}

#[kani::proof]
fn main() {
    let w = Wrapper { v: 1u8 };
    let _ = w.get();
    let _ = w.m(2u16);
    let x = 3u32;
    let r = &x;
    let _ = r.m(4u8);
}
