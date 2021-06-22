// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Make sure we can handle implicit memcpy on the empty string

fn take_string_ref(s: &str, l: usize) {
    assert!(s.len() == l)
}

fn main() {
    take_string_ref(&"x".to_string(), 1);
    take_string_ref(&"".to_string(), 0);
}
