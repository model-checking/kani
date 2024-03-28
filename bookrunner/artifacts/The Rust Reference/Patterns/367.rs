// compile-flags: --edition 2021
// kani-flags: --enable-unstable --cbmc-args --unwind 0
#![allow(unused)]
fn main() {
let words = vec!["a", "b", "c"];
let slice = &words[..];
match slice {
    [] => println!("slice is empty"),
    [one] => println!("single element {}", one),
    [head, tail @ ..] => println!("head={} tail={:?}", head, tail),
}

match slice {
    // Ignore everything but the last element, which must be "!".
    [.., "!"] => println!("!!!"),

    // `start` is a slice of everything except the last element, which must be "z".
    [start @ .., "z"] => println!("starts with: {:?}", start),

    // `end` is a slice of everything but the first element, which must be "a".
    ["a", end @ ..] => println!("ends with: {:?}", end),

    // 'whole' is the entire slice and `last` is the final element
    whole @ [.., last] => println!("the last element of {:?} is {}", whole, last),

    rest => println!("{:?}", rest),
}

if let [.., penultimate, _] = slice {
    println!("next to last is {}", penultimate);
}

let tuple = (1, 2, 3, 4, 5);
// Rest patterns may also be used in tuple and tuple struct patterns.
match tuple {
    (1, .., y, z) => println!("y={} z={}", y, z),
    (.., 5) => println!("tail must be 5"),
    (..) => println!("matches everything else"),
}
}