// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This is a custom type which is parameterized by a `usize`
pub struct Foo<const N: usize> {
    bytes: [u8; N],
}

const x: Foo<3> = Foo { bytes: [1, 2, 3] };

fn main() {
    assert!(x.bytes[0] == 1);
}
