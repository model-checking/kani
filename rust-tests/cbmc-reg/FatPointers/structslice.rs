// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Concrete {
    array: [u32; 4],
}

struct Abstract<'a> {
    uints: &'a [u32],
}

fn main() {
    let x = Concrete { array: [1, 2, 3, 4] };
    assert!(x.array[0] == 1);
    let y = Abstract { uints: &[10, 11, 12, 13] };
    assert!(y.uints[0] == 10);
    assert!(y.uints[3] == 13);
}
