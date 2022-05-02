// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// https://github.com/model-checking/kani/issues/555
// kani-flags: --no-undefined-function-checks

// Casts boxed array to boxed slice (example taken from rust documentation)
use std::str;

#[kani::proof]
fn main() {
    // This vector of bytes is used to initialize a Box<[u8; 4]>
    let sparkle_heart_vec = vec![240, 159, 146, 150];

    // This transformer produces a Box<[u8]>
    let _sparkle_heart_str = str::from_utf8(&sparkle_heart_vec);

    // see boxslice2.rs for an attempt to test sparkle_heart_str
}
