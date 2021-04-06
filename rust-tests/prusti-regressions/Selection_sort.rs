// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn selection_sort(array: &mut [i32]) {
    let mut min;

    for i in 0..array.len() {
        min = i;

        for j in (i + 1)..array.len() {
            if array[j] < array[min] {
                min = j;
            }
        }

        let tmp = array[i];
        array[i] = array[min];
        array[min] = tmp;
    }
}

fn main() {
    let mut array = [9, 4, 8, 3];
    selection_sort(&mut array);
    assert!(array == [3, 4, 8, 9]);
}
