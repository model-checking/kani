// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks whether dropping objects passed through
//! std::sync::mpsc::channel is handled.

use std::sync::mpsc::*;

static mut CELL: i32 = 0;

struct DropSetCELLToOne {}

impl Drop for DropSetCELLToOne {
    fn drop(&mut self) {
        unsafe {
            CELL = 1;
        }
    }
}

#[kani::proof]
fn main() {
    {
        let (send, recv) = channel::<DropSetCELLToOne>();
        send.send(DropSetCELLToOne {}).unwrap();
        let _to_drop: DropSetCELLToOne = recv.recv().unwrap();
    }
    assert_eq!(unsafe { CELL }, 1, "Drop should be called");
}
