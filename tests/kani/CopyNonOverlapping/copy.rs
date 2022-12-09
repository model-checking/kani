// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test verifies that std::ptr::copy_nonoverlapping works correctly
//! (originates from fixed bug https://github.com/model-checking/kani/issues/1911)

pub struct Data {
    pub t: Type,
    pub array: [u8; 8],
}

#[derive(PartialEq, Eq)]
pub enum Type {
    Apple,
    Banana,
}

fn copy_from_slice(src: &[u8], dst: &mut [u8]) {
    assert_eq!(src.len(), dst.len());
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), dst.len());
    }
}

#[kani::proof]
fn proof_harness() {
    let mut data = Data { t: Type::Apple, array: [0; 8] };
    let coin = kani::any();
    let param = [0, 0, 0, 0];
    let start = if coin { 4 } else { 0 };
    copy_from_slice(&param, &mut data.array[start..start + 4]);
    assert!(data.t == Type::Apple);
}
