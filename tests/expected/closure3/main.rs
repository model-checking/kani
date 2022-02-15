// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --output-format old
fn call_with_one<F, T>(f: F) -> T
where
    F: FnOnce(i64) -> T,
{
    f(10)
}

fn main() {
    let num: i64 = kani::any();
    if num <= std::i64::MAX - 100 {
        // avoid overflow
        let y = call_with_one(|x| x + num);
        assert!(num + 10 == y);
    }
}
