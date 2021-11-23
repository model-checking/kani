// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// From rust/library/std/src/sys_common/os_str_bytes.rs
//      rust/library/std/src/ffi/os_str.rs

use std::mem;

struct Inner {
    pub inner: [u8],
}

impl Inner {
    fn from_u8_slice(s: &[u8]) -> &Inner {
        unsafe { mem::transmute(s) }
    }
}

fn test1() {
    let inner = Inner::from_u8_slice(b"hi");
    let b = &inner.inner;
    assert!(b[0] == 'h' as u8);
    assert!(b[1] == 'i' as u8);
}

fn test2() {
    let inner = Inner::from_u8_slice(b"hi");
    assert!(inner.inner[0] == 'h' as u8);
    assert!(inner.inner[1] == 'i' as u8);
}

fn main() {
    test1();
    test2();
}
