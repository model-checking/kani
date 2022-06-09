# Checking your installation 

After you've installed Kani,
you can try running it by creating a test file:

```rust
// File: test.rs
#[kani::proof]
fn main() {
    assert!(1 == 2);
}
```

Run Kani on the single file:

```
kani test.rs
```

You should get a result like this one:

```
[...]
RESULTS:
Check 1: main.assertion.1
         - Status: FAILURE
         - Description: "assertion failed: 1 == 2"
[...]
VERIFICATION:- FAILED
```

Fix the test and you should see a result like this one:

```
[...]
VERIFICATION:- SUCCESSFUL
```

## Next steps

If you're learning Kani for the first time, you may be interested in our [tutorial](kani-tutorial.md).
