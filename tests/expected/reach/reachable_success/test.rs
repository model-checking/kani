// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --assertion-reach-checks --output-format regular --no-default-checks

fn main() {
    let x = 5;
    if x > 3 {
        assert!(x > 4);
    }
}
