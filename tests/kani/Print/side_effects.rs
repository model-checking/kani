// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that Kani's modeling of print macros takes into account any
// side effects of the arguments

fn subtract_two(x: &mut i32) -> i32 {
    let y = *x - 2;
    // side effect:
    *x = *x + 5;
    y
}

#[kani::proof]
fn main() {
    let mut x = 5;
    println!("calling function with side-effect from println!: {}", subtract_two(&mut x));
    assert!(x == 10);

    eprintln!("calling function with side-effect from eprintln!: {}", subtract_two(&mut x));
    assert!(x == 15);

    print!("calling function with side-effect from print!: {}", subtract_two(&mut x));
    assert!(x == 20);

    eprint!("calling function with side-effect from eprint!: {}", subtract_two(&mut x));
    assert!(x == 25);
}
