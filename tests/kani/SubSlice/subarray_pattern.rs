// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check binding sub-arrays with slice patterns, which used to be reported as unsupported.
//! See <https://github.com/model-checking/kani/issues/707>.

#[kani::proof]
fn subarray_middle() {
    let [x, y @ .., z] = [1, 2, 3, 4];
    assert_eq!(x, 1);
    assert_eq!(y, [2, 3]);
    assert_eq!(z, 4);
}

#[kani::proof]
fn subarray_prefix_suffix_and_empty() {
    let arr: [u8; 4] = kani::any();
    let [prefix @ .., _] = arr;
    assert_eq!(prefix, [arr[0], arr[1], arr[2]]);
    let [_, suffix @ ..] = arr;
    assert_eq!(suffix, [arr[1], arr[2], arr[3]]);
    let [a, b, c, d, empty @ ..] = arr;
    assert_eq!([a, b, c, d], arr);
    assert_eq!(empty, []);
    let [all @ ..] = arr;
    assert_eq!(all, arr);
}

#[kani::proof]
fn subarray_of_empty_array() {
    let arr: [u8; 0] = [];
    let [all @ ..] = arr;
    assert_eq!(all.len(), 0);
}

#[kani::proof]
fn subarray_write_through_mut_ref() {
    let arr: [u8; 4] = kani::any();
    let mut m = arr;
    if let [_, rest @ .., _] = &mut m {
        *rest = [7, 8];
    }
    assert_eq!(m, [arr[0], 7, 8, arr[3]]);
}

fn array() -> [(String, String); 3] {
    Default::default()
}

#[kani::proof]
fn move_out_by_subslice_and_subslice() {
    let a = array();
    let [x @ .., _, _] = a;
    let [_, ref y @ ..] = a;
    assert_eq!(x[0].0, "");
    assert_eq!(y[1].1, "");
}
