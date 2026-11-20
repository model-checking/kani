// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

static FOO: Data = Data { a: [0; 3] };

static BAR: Data = Data { b: 3 };

union Data {
    a: [u8; 3],
    b: u16,
}

#[kani::proof]
fn main() {
    let _x = &FOO;
    assert!(unsafe { FOO.a[1] } == 0);
    assert!(unsafe { BAR.b } == 3);
}
