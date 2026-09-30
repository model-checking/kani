// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

//! Test that stubbing a trait with const generic parameters produces a
//! clear error message.

trait Buf<const N: usize> {
    fn write(&self) -> usize;
}

struct MyBuf;
impl Buf<16> for MyBuf {
    fn write(&self) -> usize {
        16
    }
}

fn mock_write(_: &MyBuf) -> usize {
    42
}

#[kani::proof]
#[kani::stub(<MyBuf as Buf<16>>::write, mock_write)]
fn check_const_generic_stub() {
    let b = MyBuf;
    assert_eq!(b.write(), 42);
}
