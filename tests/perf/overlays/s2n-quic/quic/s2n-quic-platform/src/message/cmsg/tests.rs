// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use bolero::{check, TypeGenerator};
use core::mem::align_of;
use libc::c_int;

#[inline]
fn aligned_iter(bytes: &[u8], f: impl FnOnce(decode::Iter)) {
    // the bytes needs to be aligned to a cmsghdr
    let offset = bytes.as_ptr().align_offset(align_of::<cmsghdr>());

    if let Some(bytes) = bytes.get(offset..) {
        let iter = unsafe {
            // SAFETY: bytes are aligned above
            decode::Iter::from_bytes(bytes)
        };

        f(iter)
    }
}

/// Ensures the cmsg iterator doesn't crash or segfault
#[test]
#[cfg_attr(kani, kani::proof, kani::solver(minisat), kani::unwind(17))]
fn iter_test() {
    check!().for_each(|bytes| {
        aligned_iter(bytes, |iter| {
            for (cmsghdr, value) in iter {
                let _ = cmsghdr;
                let _ = value;
            }
        })
    });
}

/// Ensures the `decode::Iter::collect` doesn't crash or segfault
#[test]
#[cfg_attr(kani, kani::proof, kani::solver(minisat), kani::unwind(17))]
fn collect_test() {
    check!().for_each(|bytes| {
        aligned_iter(bytes, |iter| {
            let _ = iter.collect();
        })
    });
}

#[derive(Clone, Copy, Debug, TypeGenerator)]
struct Op {
    level: c_int,
    ty: c_int,
    value: Value,
}

#[derive(Clone, Copy, Debug, TypeGenerator)]
enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    // alignment can't exceed that of cmsghdr
    U64([u32; 2]),
    U128([u32; 4]),
}

impl Value {
    fn check_value(&self, bytes: &[u8]) {
        let expected_len = match self {
            Self::U8(_) => 1,
            Self::U16(_) => 2,
            Self::U32(_) => 4,
            Self::U64(_) => 8,
            Self::U128(_) => 16,
        };
        assert_eq!(expected_len, bytes.len());
    }
}

fn round_trip(ops: &[Op]) {
    let mut storage = Storage::<32>::default();
    let mut encoder = storage.encoder();

    let mut expected_encoded_count = 0;

    for op in ops {
        let res = match op.value {
            Value::U8(value) => encoder.encode_cmsg(op.level, op.ty, value),
            Value::U16(value) => encoder.encode_cmsg(op.level, op.ty, value),
            Value::U32(value) => encoder.encode_cmsg(op.level, op.ty, value),
            Value::U64(value) => encoder.encode_cmsg(op.level, op.ty, value),
            Value::U128(value) => encoder.encode_cmsg(op.level, op.ty, value),
        };

        match res {
            Ok(_) => expected_encoded_count += 1,
            Err(_) => break,
        }
    }

    let mut actual_decoded_count = 0;
    let mut iter = encoder.iter();

    for (op, (cmsghdr, value)) in ops.iter().zip(&mut iter) {
        assert_eq!(op.level, cmsghdr.cmsg_level);
        assert_eq!(op.ty, cmsghdr.cmsg_type);
        op.value.check_value(value);
        actual_decoded_count += 1;
    }

    assert_eq!(expected_encoded_count, actual_decoded_count);
    assert!(iter.next().is_none());
}

#[cfg(not(kani))]
type Ops = Vec<Op>;
#[cfg(kani)]
type Ops = s2n_quic_core::testing::InlineVec<Op, 8>;

#[test]
#[cfg_attr(kani, kani::proof, kani::solver(kissat), kani::unwind(9))]
fn round_trip_test() {
    check!().with_type::<Ops>().for_each(|ops| round_trip(ops));
}
