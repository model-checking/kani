// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend declares the regions of function signatures the way
//! Charon's type check expects: every late-bound region, wherever it occurs in the signature and
//! only once; late-bound regions numbered after early-bound ones; generic parameters numbered per
//! kind; and the regions bound by function-pointer types. Each of these used to make Charon
//! reject the translation.

struct Holder<'a> {
    r: &'a u32,
}

impl<'a> Holder<'a> {
    // `'a` is early-bound (from the impl), the region of `&self` late-bound.
    fn get(&self) -> u32 {
        *self.r
    }
}

// A late-bound region nested in an ADT.
fn get(x: Option<&u32>) -> u32 {
    match x {
        Some(v) => *v,
        None => 0,
    }
}

// Two late-bound regions, one nested in the other.
fn deref2(x: &&u32) -> u32 {
    **x
}

// A late-bound region as an ADT's region argument.
fn read(h: Holder<'_>) -> u32 {
    *h.r
}

// One late-bound region used three times.
fn pick<'a>(x: &'a u32, y: &'a u32, c: bool) -> &'a u32 {
    if c { x } else { y }
}

// An early-bound region (because of `T: 'a`) before a type parameter.
fn early<'a, T: 'a>(x: &'a T) -> &'a T {
    x
}

// A function-pointer type binding its own region.
fn take(g: Option<fn(&u32) -> &u32>, x: &u8) -> u8 {
    let _ = g;
    *x
}

#[kani::proof]
fn main() {
    let a = 1u32;
    let b = 2u32;
    let r = &a;
    let h = Holder { r: &a };
    let _ = h.get();
    let _ = get(Some(&a));
    let _ = deref2(&r);
    let _ = read(Holder { r: &b });
    let _ = *pick(&a, &b, true);
    let _ = *early(&a);
    let c = 3u8;
    let _ = take(None, &c);
}
