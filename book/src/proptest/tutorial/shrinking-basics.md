# Shrinking Basics

Finding the "simplest" input that causes a test failure is referred to as
_shrinking_. This is where the intermediate `ValueTree` type comes in.
Besides `current()`, it provides two methods — `simplify()` and
`complicate()` — which together allow binary searching over the input
space. The `tutorial-simplify-play.rs` example shows how repeated calls to
`simplify()` produce incrementally "simpler" outputs, both in terms of size
and in characters used.

```rust
use proptest::test_runner::TestRunner;
use proptest::strategy::{Strategy, ValueTree};

fn main() {
    let mut runner = TestRunner::default();
    let mut str_val = "[a-z]{1,4}\\p{Cyrillic}{1,4}\\p{Greek}{1,4}"
        .new_tree(&mut runner).unwrap();
    println!("str_val = {}", str_val.current());
    while str_val.simplify() {
        println!("        = {}", str_val.current());
    }
}
```

A couple runs:

```text
$ target/debug/examples/tutorial-simplify-play
str_val = vy꙲ꙈᴫѱΆῨῨ
        = y꙲ꙈᴫѱΆῨῨ
        = y꙲ꙈᴫѱΆῨῨ
        = m꙲ꙈᴫѱΆῨῨ
        = g꙲ꙈᴫѱΆῨῨ
        = d꙲ꙈᴫѱΆῨῨ
        = b꙲ꙈᴫѱΆῨῨ
        = a꙲ꙈᴫѱΆῨῨ
        = aꙈᴫѱΆῨῨ
        = aᴫѱΆῨῨ
        = aѱΆῨῨ
        = aѱΆῨῨ
        = aѱΆῨῨ
        = aиΆῨῨ
        = aМΆῨῨ
        = aЎΆῨῨ
        = aЇΆῨῨ
        = aЃΆῨῨ
        = aЁΆῨῨ
        = aЀΆῨῨ
        = aЀῨῨ
        = aЀῨ
        = aЀῨ
        = aЀῢ
        = aЀ῟
        = aЀ῞
        = aЀ῝
$ target/debug/examples/tutorial-simplify-play
str_val = dyiꙭᾪῇΊ
        = yiꙭᾪῇΊ
        = iꙭᾪῇΊ
        = iꙭᾪῇΊ
        = iꙭᾪῇΊ
        = eꙭᾪῇΊ
        = cꙭᾪῇΊ
        = bꙭᾪῇΊ
        = aꙭᾪῇΊ
        = aꙖᾪῇΊ
        = aꙋᾪῇΊ
        = aꙅᾪῇΊ
        = aꙂᾪῇΊ
        = aꙁᾪῇΊ
        = aꙀᾪῇΊ
        = aꙀῇΊ
        = aꙀΊ
        = aꙀΊ
        = aꙀΊ
        = aꙀΉ
        = aꙀΈ
```

Note that shrinking never shrinks a value to something outside the range
the strategy describes. Notice the strings in the above example still match
the regular expression even in the end. An integer drawn from
`100..1000i32` will shrink towards zero, but will stop at 100 since that is
the minimum value.

`simplify()` and `complicate()` can be used to adapt our primitive fuzz
test to actually find the boundary condition.

```rust
use proptest::test_runner::TestRunner;
use proptest::strategy::{Strategy, ValueTree};

fn some_function(v: i32) -> bool {
    // Do a bunch of stuff, but crash if v > 500
    // assert!(v <= 500);
    // But return a boolean instead of panicking for simplicity
    v <= 500
}

// We know the function is broken, so use a purpose-built main function to
// find the breaking point.
fn main() {
    let mut runner = TestRunner::default();
    for _ in 0..256 {
        let mut val = (0..10000i32).new_tree(&mut runner).unwrap();
        if some_function(val.current()) {
            // Test case passed
            continue;
        }

        // We found our failing test case, simplify it as much as possible.
        loop {
            if !some_function(val.current()) {
                // Still failing, find a simpler case
                if !val.simplify() {
                    // No more simplification possible; we're done
                    break;
                }
            } else {
                // Passed this input, back up a bit
                if !val.complicate() {
                    break;
                }
            }
        }

        println!("The minimal failing case is {}", val.current());
        assert_eq!(501, val.current());
        return;
    }
    panic!("Didn't find a failing test case");
}
```

This code reliably finds the boundary of the failure, 501.
