// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Fn-bounded generic functions: previously skipped ("no candidate type satisfies the
// function's trait bounds"), now instantiated with nondeterministic function items.

// TEST NOTE: harnessed as apply::<u8, nondet_fn1 item>; PASSES (wrapping arithmetic).
pub fn apply<F: Fn(u8) -> u8>(f: F, x: u8) -> u8 {
    f(x).wrapping_add(1)
}

// TEST NOTE: FAILS: the closure result is unconstrained, so the addition can overflow —
// a real bug class in the generic function's own code, found for ANY closure behavior.
pub fn apply_buggy<F: Fn(u8) -> u8>(f: F, x: u8) -> u8 {
    f(x) + 1
}

// TEST NOTE: FnMut with two arguments; the fold-style accumulation overflows: FAILS.
pub fn fold2<F: FnMut(u32, u32) -> u32>(mut f: F, a: u32, b: u32) -> u32 {
    f(a, b) + f(b, a)
}

// TEST NOTE: FnOnce returning unit: PASSES (nothing to go wrong).
pub fn run_once<F: FnOnce() -> ()>(f: F) {
    f()
}

// TEST NOTE: cover check must be SATISFIED: the nondet closure's results genuinely cover
// the range (both branches reachable).
pub fn branches<F: Fn(u8) -> bool>(f: F, x: u8) {
    if f(x) {
        kani::cover!(true, "true branch reachable");
    } else {
        kani::cover!(true, "false branch reachable");
    }
}

// TEST NOTE: harnessed as apply_generic::<i32, nondet_fn1<i32, i32>>: the closure
// signature references another generic parameter, resolved per candidate choice.
pub fn apply_generic<T, F: Fn(T) -> T>(f: F, x: T) -> T {
    f(x)
}

// TEST NOTE (regression, tap ICE): HRTB closure bound (for<'a> via &Self sugar);
// previously leaked escaping bound vars into the trait solver.
pub fn inspect_with<F: FnOnce(&u32)>(f: F, v: u32) {
    f(&v);
}

// TEST NOTE (regression, nom ICE): enum variant holding an anonymous tuple field;
// previously the derive-style generator had no tuple vocabulary.
pub enum Packet {
    Pair((u8, u16)),
    Empty,
}
pub fn packet_size(p: Packet) -> usize {
    match p {
        Packet::Pair((a, _)) => a as usize,
        Packet::Empty => 0,
    }
}

// TEST NOTE: top-level anonymous tuple argument, generated elementwise.
pub fn tuple_arg(t: (u8, bool)) -> u8 {
    if t.1 { t.0 } else { 0 }
}

// TEST NOTE (regression, reqwest ICE): HRTB closure param wrapped in a struct and
// coerced to a trait object. Requires the region-polymorphic nondet_fn1_ref model
// (an early-bound fn item leaves the method slot vacant in the concrete vtable).
struct ScopeFn<F>(F);
trait Scope {
    fn run(&self) -> bool;
}
impl<F: Fn(&u8) -> bool> Scope for ScopeFn<F> {
    fn run(&self) -> bool {
        (self.0)(&7)
    }
}
pub fn scoped<F: Fn(&u8) -> bool + 'static>(f: F) -> bool {
    let s: Box<dyn Scope> = Box::new(ScopeFn(f));
    s.run()
}
