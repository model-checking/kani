// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let numbers = vec![1, 2, 3, 4, 5];

    // Attempt to access the 10th element of the vector, which is out of bounds.
    let tenth_element = numbers.get(9);

    if let Some(value) = tenth_element {
        // This part is unreachable since the vector has only 5 elements (indices 0 to 4).
        println!("The 10th element is: {}", value);
    } else {
        println!("The 10th element is out of bounds!");
    }
}
