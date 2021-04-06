// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// From rust/library/std/src/sys_common/os_str_bytes.rs
//      rust/library/std/src/ffi/os_str.rs

use std::mem;

struct Slice {
    pub inner: [u8],
}

impl Slice {
    fn from_u8_slice(s: &[u8]) -> &Slice {
        unsafe { mem::transmute(s) }
    }
    pub fn from_str(s: &str) -> &Slice {
        Slice::from_u8_slice(s.as_bytes())
    }
}

struct OsStr {
    inner: Slice,
}

impl AsRef<OsStr> for str {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        OsStr::from_inner(Slice::from_str(self))
    }
}

impl OsStr {
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &OsStr {
        s.as_ref()
    }
    fn as_inner(&self) -> &Slice {
        &self.inner
    }
    fn from_inner(inner: &Slice) -> &OsStr {
        unsafe { &*(inner as *const Slice as *const OsStr) }
    }
    fn as_bytes(&self) -> &[u8] {
        &self.as_inner().inner
    }
}

fn main() {
    let x = OsStr::new("hi");
    x.as_bytes();
}
