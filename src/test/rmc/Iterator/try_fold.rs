// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// cbmc-flags: --unwind 3

fn main() {
    let arr = [(1, 2), (2, 2)];
    let result = arr.iter().try_fold((), |acc, &i| Some(()));
    assert_ne!(result, None, "This should succeed");
}
