// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks the `&Wtf8` model used by autoharness at a bound that holds two surrogates: it generates
//! unpaired surrogates, and never a lead surrogate directly followed by a trail surrogate, which
//! is not well-formed WTF-8.

#![feature(wtf8_internals)]
#![allow(internal_features)]

#[kani::proof]
#[kani::unwind(7)]
fn check_surrogates() {
    let mut storage: [u8; 6] = kani::any();
    let s = kani::any_wtf8_ref(&mut storage);
    let b = s.as_bytes();
    // `ED` never continues a sequence, so each `ED` below starts a code point.
    for i in 3..b.len() {
        let after_lead = b[i - 3] == 0xED && (0xA0..=0xAF).contains(&b[i - 2]);
        let trail = b[i] == 0xED && b.get(i + 1).is_some_and(|&c| c >= 0xB0);
        kani::assert(!(after_lead && trail), "lead surrogate followed by a trail surrogate");
    }
    kani::cover!(s.final_lead_surrogate().is_some(), "ends in a lead surrogate");
    kani::cover!(s.initial_trail_surrogate().is_some(), "starts with a trail surrogate");
    kani::cover!(
        b.len() == 6 && s.initial_trail_surrogate().is_some() && s.final_lead_surrogate().is_some(),
        "trail surrogate followed by a lead surrogate"
    );
}
