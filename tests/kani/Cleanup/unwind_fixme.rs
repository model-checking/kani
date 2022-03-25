// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// We currently do not support stack unwinding panic strategy. Once we do, this testcase should
// fail during the verification with both the panic and the assertion failing.
// https://github.com/model-checking/kani/issues/692

// compile-flags: --C panic=unwind --crate-type lib
// kani-verify-fail

pub struct DummyResource {
    pub data: Option<String>,
}

impl Drop for DummyResource {
    fn drop(&mut self) {
        assert!(self.data.is_some(), "This should fail");
    }
}

#[kani::proof]
pub fn create(empty: bool) -> DummyResource {
    let mut dummy = DummyResource { data: None };
    if empty {
        unimplemented!("This is not supported yet");
    }
    dummy.data = Some(String::from("data"));
    dummy
}
