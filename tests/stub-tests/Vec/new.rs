// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn new_test() {
        let v: Vec<i32> = Vec::new();
    }

    new_test();
}
