// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks coverage results in an example with a `match` statement matching on
//! all enum variants.

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn print_direction(dir: Direction) {
    match dir {
        Direction::Up => println!("Going up!"),
        Direction::Down => println!("Going down!"),
        Direction::Left => println!("Going left!"),
        Direction::Right if 1 == 1 => println!("Going right!"),
        // This part is unreachable since we cover all variants in the match.
        _ => println!("Not going anywhere!"),
    }
}

#[kani::proof]
fn main() {
    let direction = Direction::Left;
    print_direction(direction);
}
