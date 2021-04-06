// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut door_open = [false; 10];
    for pass in 1..11 {
        let mut door = pass;
        while door <= 10 {
            door_open[door - 1] = !door_open[door - 1];
            door += pass;
        }
    }

    for i in 1..4 {
        let idx = i * i - 1;
        assert!(door_open[idx]);
        door_open[idx] = false;
    }

    for i in 1..4 {
        assert!(!door_open[i]);
    }
}
