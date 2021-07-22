// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-verify-fail

// Casts boxed array to boxed slice (example taken from rust documentation)
use std::str;

fn main() {
    // This vector of bytes is used to initialize a Box<[u8; 4]>
    let sparkle_heart_vec = vec![240, 159, 146, 150];

    // This transformer produces a Box<[u8]>
    let sparkle_heart_str = str::from_utf8(&sparkle_heart_vec);

    // This match statement generates failures even though
    // the binary runs without failures.
    match sparkle_heart_str {
        Ok(string) => match string.bytes().nth(0) {
            Some(b) => assert!(b == 240),
            _ => assert!(true),
        },
        _ => assert!(true),
    }
}
