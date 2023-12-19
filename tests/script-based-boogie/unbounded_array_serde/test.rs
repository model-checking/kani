// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A test that demonstrates unbounded verification of array-based programs.
//! The test uses `any_array` which creates arrays with non-deterministic
//! content and length.
//! The `src` array of words is serialized into the `buf` byte array and then
//! deserialized back into `dst`.
//! The test checks that the round trip of serialization and deserialization is
//! the identity.

#[kani::proof]
fn serde() {
    let src = kani::array::any_array::<u16>();
    let mut buf = kani::array::any_array::<u8>();
    let src_len: usize = src.len();
    let buf_len: usize = buf.len();
    kani::assume(buf_len >> 1u64 >= src_len);

    // serialize
    let mut i: usize = 0;
    //Loop_invariant: forall j: usize :: j < i => ((buf[j << 1u64 + 1] ++ buf[j << 1u64]) == src[j])
    while i < src_len {
        let x = src[i];
        let byte0: u8 = (x & 0xFF) as u8;
        let byte1: u8 = ((x >> 8u16) & 0xFF) as u8;
        let j: usize = i << 1u64;
        buf[j] = byte0;
        buf[j + 1] = byte1;
        i += 1;
    }

    // deserialize
    let mut dst = kani::array::any_array::<u16>();
    kani::assume(dst.len() >= src_len);
    i = 0;
    //Loop_invariant: forall j: usize :: j < i => ((buf[j << 1u64 + 1] ++ buf[j << 1u64]) == dst[j])
    while i < src_len {
        let j: usize = i << 1u64;
        dst[i] = ((buf[j + 1] as u16) << 8u16) | (buf[j] as u16);
        i += 1;
    }

    // Check the round trip
    i = 0;
    while i < src_len {
        kani::assert(src[i] == dst[i], "serialization/deserialization failed");
        i += 1;
    }
}
