// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    let cond: bool = kani::any();
    kani::assume(cond);
    assert!(cond);
}
