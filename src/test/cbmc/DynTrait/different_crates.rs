// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

struct Counter {
    count: usize,
}

impl std::iter::Iterator for Counter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        // Increment our count. 
        self.count += 1;
        Some(self.count)
    }
}

trait Iterator {
    fn next(&mut self) -> Option<usize>;
}

impl Iterator for Counter {
    fn next(&mut self) -> Option<usize> {
        Some(42)
    }
}

trait Combined : Iterator + std::iter::Iterator<Item = usize> {}

impl Combined for Counter {

}

fn std_count(c : &mut dyn std::iter::Iterator<Item = usize>) -> usize {
    c.next().unwrap()
}

fn weird_count(c : &mut dyn Iterator) -> usize {
    c.next().unwrap()
}


fn main() {
    let counter : &mut Counter = &mut Counter { count: 0 };
    assert!(std_count(counter as &mut dyn std::iter::Iterator<Item = usize>) == 1);
    assert!(weird_count(counter as &mut dyn Iterator) == 42);
    
    let counter_combined = counter as &mut dyn Combined;
    assert!(std::iter::Iterator::next(counter_combined).unwrap() == 2);
    assert!(Iterator::next(counter_combined).unwrap() == 42);
}