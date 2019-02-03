# Forking and Timeouts

By default, proptest tests are run in-process and are allowed to run for
however long it takes them. This is resource-efficient and produces the nicest
test output, and for many use cases is sufficient. However, problems like
overflowing the stack, aborting the process, or getting stuck in an infinite
loop will simply break the entire test process and prevent proptest from
determining a minimal reproducible case.

As of version 0.7.1, proptest has optional "fork" and "timeout" features
(both enabled by default), which make it possible to run your test cases in
a subprocess and limit how long they may run. This is generally slower,
may make using a debugger more difficult, and makes test output harder to
interpret, but allows proptest to find and minimise test cases for these
situations as well.

To use these features, simply set the `fork` and/or `timeout` fields on the
`Config`. (Setting `timeout` implies `fork`.)

Here is a simple example of using both features:

```rust
use proptest::prelude::*;

// The worst possible way to calculate Fibonacci numbers
fn fib(n: u64) -> u64 {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Setting both fork and timeout is redundant since timeout implies
        // fork, but both are shown for clarity.
        fork: true,
        timeout: 1000,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_fib(n: u64) {
        // For large n, this will variously run for an extremely long time,
        // overflow the stack, or panic due to integer overflow.
        assert!(fib(n) >= n);
    }
}
# //NOREADME
# fn main() { } //NOREADME
```

The exact value of the test failure depends heavily on the performance of
the host system, the rust version, and compiler flags, but on the system
where it was originally tested, it found that the maximum value that
`fib()` could handle was 39, despite having dozens of processes dump core
due to stack overflow or time out along the way.

If you just want to run tests in subprocesses or with a timeout every now
and then, you can do that by setting the `PROPTEST_FORK` or
`PROPTEST_TIMEOUT` environment variables to alter the default
configuration. For example, on Unix,

```sh
# Run all the proptest tests in subprocesses with no timeout.
# Individual tests can still opt out by setting `fork: false` in their
# own configuration.
PROPTEST_FORK=true cargo test
# Run all the proptest tests in subprocesses with a 1 second timeout.
# Tests can still opt out or use a different timeout by setting `timeout: 0`
# or another timeout in their own configuration.
PROPTEST_TIMEOUT=1000 cargo test
```
