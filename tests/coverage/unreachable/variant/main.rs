// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

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
        Direction::Right => println!("Going right!"),
        // This part is unreachable since we cover all variants in the match.
    }
}

#[kani::proof]
fn main() {
    let direction = Direction::Left;
    print_direction(direction);
}
