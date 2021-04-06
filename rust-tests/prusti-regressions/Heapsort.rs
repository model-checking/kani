// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut v = [3, 0, 1, 2];
    create_heap(&mut v, |x, y| x < y);
    assert!(v == [3, 2, 1, 0]);
}

fn create_heap<T, F>(array: &mut [T], order: F)
where
    F: Fn(&T, &T) -> bool,
{
    let len = array.len();
    // Create heap
    for start in (0..len / 2).rev() {
        shift_down(array, &order, start, len - 1)
    }
}

/// not running heap sort because it's too slow
fn heap_sort<T, F>(array: &mut [T], order: F)
where
    F: Fn(&T, &T) -> bool,
{
    let len = array.len();
    // Create heap
    for start in (0..len / 2).rev() {
        shift_down(array, &order, start, len - 1)
    }

    for end in (1..len).rev() {
        array.swap(0, end);
        shift_down(array, &order, 0, end - 1)
    }
}

fn shift_down<T, F>(array: &mut [T], order: &F, start: usize, end: usize)
where
    F: Fn(&T, &T) -> bool,
{
    let mut root = start;
    loop {
        let mut child = root * 2 + 1;
        if child > end {
            break;
        }
        if child + 1 <= end && order(&array[child], &array[child + 1]) {
            child += 1;
        }
        if order(&array[root], &array[child]) {
            array.swap(root, child);
            root = child
        } else {
            break;
        }
    }
}
