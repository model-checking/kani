// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// This test checks that CBMC's unwinding terminates on a program that involves
/// dyn trait and `char::encode_utf8` (a minimal reproducer from
/// https://github.com/model-checking/kani/issues/1767)

pub trait Trait {
    fn f(&self);
}

struct Foo {}

impl Trait for Foo {
    fn f(&self) {
        let _ = 'x'.encode_utf8(&mut [0; 4]);
    }
}

pub struct Formatter {}

fn nn(_x: &u8, _f: &Formatter) {}

pub struct ArgumentV1 {
    formatter: fn(&u8, &Formatter) -> (),
}

#[kani::proof]
#[kani::unwind(2)]
fn dyn_trait_with_encode_utf8() {
    let f = Foo {};
    let a = [ArgumentV1 { formatter: nn }];

    let _output = &f as &dyn Trait;
    let formatter = Formatter {};

    let mut iter = a.iter();
    let _ = (iter.next().unwrap().formatter)(&5, &formatter);
}
