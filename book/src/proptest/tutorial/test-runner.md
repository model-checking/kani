# Using the Test Runner

Rather than manually shrinking, proptest's
[`TestRunner`](test_runner/struct.TestRunner.html) provides this
functionality for us and additionally handles things like panics. The
method we're interested in is `run`. We simply
give it the strategy and a function to test inputs and it takes care of the
rest.

```rust
use proptest::test_runner::{Config, FileFailurePersistence,
                            TestError, TestRunner};

fn some_function(v: i32) {
    // Do a bunch of stuff, but crash if v > 500.
    // We return to normal `assert!` here since `TestRunner` catches
    // panics.
    assert!(v <= 500);
}

// We know the function is broken, so use a purpose-built main function to
// find the breaking point.
fn main() {
    let mut runner = TestRunner::new(Config {
        // Turn failure persistence off for demonstration
        failure_persistence: Some(Box::new(FileFailurePersistence::Off)),
        .. Config::default()
    });
    let result = runner.run(&(0..10000i32), |v| {
        some_function(v);
        Ok(())
    });
    match result {
        Err(TestError::Fail(_, value)) => {
            println!("Found minimal failing case: {}", value);
            assert_eq!(501, value);
        },
        result => panic!("Unexpected result: {:?}", result),
    }
}
```

That's a lot better! Still a bit boilerplatey; the `proptest!` macro will
help with that, but it does some other stuff we haven't covered yet, so for
the moment we'll keep using `TestRunner` directly.
