// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test a reference to a subslice of the trailing slice field of an unsized struct. The subslice
//! place used to be represented by its fat pointer, and taking a reference to it made codegen
//! panic with "Can't take address of Expr".

pub struct Wrapper<T: ?Sized> {
    pub n: u8,
    pub data: T,
}

fn data_tail(w: &Wrapper<[u8]>) -> &[u8] {
    match w.data {
        [_, ref tail @ ..] => tail,
        [] => &[],
    }
}

#[kani::proof]
fn check_dst_field_subslice() {
    let concrete: Wrapper<[u8; 3]> = Wrapper { n: 0, data: kani::any() };
    let w: &Wrapper<[u8]> = &concrete;
    let tail = data_tail(w);
    assert_eq!(tail.len(), 2);
    assert_eq!(tail[0], concrete.data[1]);
    assert_eq!(tail[1], concrete.data[2]);
}
