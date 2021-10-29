// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --use-abs --abs-type rmc

static mut GLOB: i32 = 1;

struct Test {
    _marker: u32
}

impl Drop for Test {
    fn drop(&mut self) {
        unsafe {
            GLOB += 1;
        }
    }
}

fn main() {
    fn drop_test() {
        {
            let mut v = Vec::new();
            v.push(Test { _marker: 0 });
            v.push(Test { _marker: 0 });
        }

        unsafe {
            assert!(GLOB == 3);
        }
    }

    drop_test();
}
