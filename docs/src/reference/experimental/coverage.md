## Coverage

Recall our `estimate_size` example from [First steps](../tutorial-first-steps.md),
where we wrote a proof harness constraining the range of inputs to integers less than 4096:

```rust
{{#include ../../tutorial/first-steps-v2/src/lib.rs:kani}}
```

We must wonder if we've really fully tested our function.
What if we revise the function, but forget to update the assumption in our proof harness to cover the new range of inputs?

Fortunately, Kani is able to report a coverage metric for each proof harness.
In the `first-steps-v2` directory, try running:

```
cargo kani --coverage -Z line-coverage --harness verify_success
```

which verifies the harness, then prints coverage information for each line.
In this case, we see that each line of `estimate_size` is followed by `FULL`, indicating that our proof harness provides full coverage.

Try changing the assumption in the proof harness to `x < 2048`.
Now the harness won't be testing all possible cases.
Rerun the command.
You'll see this line:

```
src/lib.rs, 24, NONE
```

which indicates that the proof no longer covers line 24, which addresses the case where `x >= 2048`.
