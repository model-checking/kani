// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// FIXME: a higher-ranked fn-pointer target mis-resolves to a coexisting `'static`
// sibling impl. `resolve_ty` erases the fn pointer's lifetimes, so
// `<Wrap<fn(&u8) -> u8> as Probe>::probe` resolves to the `fn(&'static u8) -> u8`
// impl instead of its own, and `h_hr` cannot reach its target. Binding the late-bound
// regions (so the higher-ranked type stays distinct) is tracked at
// https://github.com/model-checking/kani/issues/4933. Both impls can coexist (rustc
// only warns with `coherence_leak_check`).
#![allow(coherence_leak_check)]

struct Wrap<T>(T);
trait Probe {
    fn probe(&self) -> u32;
}

impl Probe for Wrap<fn(&u8) -> u8> {
    #[kani::ensures(|r| *r == 7)]
    fn probe(&self) -> u32 {
        7
    }
}

impl Probe for Wrap<fn(&'static u8) -> u8> {
    #[kani::ensures(|r| *r == 9)]
    fn probe(&self) -> u32 {
        9
    }
}

#[kani::proof_for_contract(<Wrap<fn(&u8) -> u8> as Probe>::probe)]
fn h_hr() {
    fn d(x: &u8) -> u8 {
        *x
    }
    let _ = Wrap(d as fn(&u8) -> u8).probe();
}
