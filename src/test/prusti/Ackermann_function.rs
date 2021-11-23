// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn ack(m: u64, n: u64) -> u64 {
    match (m, n) {
        (0, n) => n + 1,
        (m, 0) => ack(m - 1, 1),
        (m, n) => ack(m - 1, ack(m, n - 1)),
    }
}

fn main() {
    let a = ack(2, 4);
    assert!(a == 11);
}
