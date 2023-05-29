# First steps

Kani is unlike the testing tools you may already be familiar with.
Much of testing is concerned with thinking of new corner cases that need to be covered.
With Kani, all the corner cases are covered from the start, and the new concern is narrowing down the scope to something manageable for the verifier.

Consider this first program (which can be found under [`first-steps-v1`](https://github.com/model-checking/kani/tree/main/docs/src/tutorial/first-steps-v1/)):

```rust
{{#include tutorial/first-steps-v1/src/lib.rs:code}}
```

Think about the test harness you would need to write to test this function.
You would need figure out a whole set of arguments to call the function with that would exercise each branch.
You would also need to keep that test harness up-to-date with the code, in case some of the branches change.
And if this function was more complicated—for example, if some of the branches depended on global state—the test harness would be even more onerous to write.

We can try to property test a function like this, but if we're naive about it (and consider all possible `u32` inputs), then it's unlikely we'll ever find the bug.

```rust
{{#include tutorial/first-steps-v1/src/lib.rs:proptest}}
```

```
# cargo test
[...]
test tests::doesnt_crash ... ok
```

There's only 1 in 4 billion inputs that fail, so it's vanishingly unlikely the property test will find it, even with a million samples.

Let's write a Kani [_proof harness_](reference/attributes.md#kaniproof) for `estimate_size`.
This is a lot like a test harness, but now we can use `kani::any()` to represent all possible `u32` values:

```rust
{{#include tutorial/first-steps-v1/src/lib.rs:kani}}
```

```
# cargo kani
[...]
Runtime decision procedure: 0.00116886s

RESULTS:
Check 3: estimate_size.assertion.1
         - Status: FAILURE
         - Description: "Oh no, a failing corner case!"
[...]
VERIFICATION:- FAILED
```

Kani has immediately found a failure.
Notably, we haven't had to write explicit assertions in our proof harness: by default, Kani will find a host of erroneous conditions which include a reachable call to `panic` or a failing `assert`.
If Kani had run successfully on this harness, this amounts to a mathematical proof that there is no input that could cause a panic in `estimate_size`.

### Getting a trace

By default, Kani only reports failures, not how the failure happened.
In this running example, it seems obvious what we're interested in (the value of `x` that caused the failure) because we just have one unknown input at the start (similar to the property test), but that's kind of a special case.
In general, understanding how a failure happened requires exploring a full (potentially large) _execution trace_.

An execution trace is a record of exactly how a failure can occur.
Nondeterminism (like a call to `kani::any()`, which could return any value) can appear in the middle of its execution.
A trace is a record of exactly how execution proceeded, including concrete choices (like `1023`) for all of these nondeterministic values.

To get a trace for a failing check in Kani, run:

```
kani test.rs --visualize --enable-unstable
```

This command runs Kani and generates an HTML report that includes a trace.
Open the report with your preferred browser.
Under the "Errors" heading, click on the "trace" link to find the trace for this failure.

From this trace report, we can filter through it to find relevant lines.
A good rule of thumb is to search for either `kani::any()` or assignments to variables you're interested in.
At present time, an unfortunate amount of generated code is present in the trace.
This code isn't a part of the Rust code you wrote, but is an internal implementation detail of how Kani runs proof harnesses.
Still, searching for `kani::any()` quickly finds us these lines:

```
let x: u32 = kani::any();
x = 1023u
```

Here we're seeing the line of code and the value assigned in this particular trace.
Like property testing, this is just one **example** of a failure.
To proceed, we recommend fixing the code to avoid this particular issue and then re-running Kani to see if you find more issues.

### Exercise: Try other failures

We put an explicit panic in this function, but it's not the only kind of failure Kani will find.
Try a few other types of errors.

For example, instead of panicking we could try explicitly dereferencing a null pointer:

```rust
unsafe { return *(0 as *const u32) };
```

Notably, however, the Rust compiler emits a warning here:

```
warning: dereferencing a null pointer
  --> src/lib.rs:10:29
   |
10 |    unsafe { return *(0 as *const u32) };
   |                    ^^^^^^^^^^^^^^^^^^ this code causes undefined behavior when executed
   |
   = note: `#[warn(deref_nullptr)]` on by default
```

Still, it's just a warning, and we can run the code without test failures just as before.
But Kani still catches the issue:

```
[...]
RESULTS:
[...]
Check 2: estimate_size.pointer_dereference.1
         - Status: FAILURE
         - Description: "dereference failure: pointer NULL"
[...]
VERIFICATION:- FAILED
```

**Exercise: Can you find an example where the Rust compiler will not complain, and Kani will?**

<details>
<summary>Click to show one possible answer</summary>

```
return 1 << x;
```

Overflow (in addition, multiplication or, in this case, [bit-shifting by too much](https://github.com/rust-lang/rust/issues/10183)) is also caught by Kani:

```
RESULTS:
[...]
Check 1: estimate_size.assertion.1
         - Status: FAILURE
         - Description: "attempt to shift left with overflow"

Check 3: estimate_size.undefined-shift.1
         - Status: FAILURE
         - Description: "shift distance too large"
[...]
VERIFICATION:- FAILED
```

</details>

## Assertions, Assumptions, and Harnesses

It seems a bit odd that our example function is tested against billions of possible inputs, when it really only seems to be designed to handle a few thousand.
Let's encode this fact about our function by asserting some reasonable upper bound on our input, after we've fixed our bug.
(New code available under [`first-steps-v2`](https://github.com/model-checking/kani/tree/main/docs/src/tutorial/first-steps-v2/)):

```rust
{{#include tutorial/first-steps-v2/src/lib.rs:code}}
```

Now we've explicitly stated our previously implicit expectation: this function should never be called with inputs that are too big.
But if we attempt to verify this modified function, we run into a problem:

```
[...]
RESULTS:
[...]
Check 3: estimate_size.assertion.1
         - Status: FAILURE
         - Description: "assertion failed: x < 4096"
[...]
VERIFICATION:- FAILED
```

What we want is a _precondition_ for `estimate_size`.
That is, something that should always be true every time we call the function.
By putting the assertion at the beginning, we ensure the function immediately fails if that expectation is not met.

But our proof harness will still call this function with any integer, even ones that just don't meet the function's preconditions.
That's... not a useful or interesting result.
We know that won't work already.
How do we go back to successfully verifying this function?

This is the purpose of writing a proof harness.
Much like property testing (which would also fail in this assertion), we need to set up our preconditions, call the function in question, then assert our postconditions.
Here's a revised example of the proof harness, one that now succeeds:

```rust
{{#include tutorial/first-steps-v2/src/lib.rs:kani}}
```

But now we must wonder if we've really fully tested our function.
What if we revise the function, but forget to update the assumption in our proof harness to cover the new range of inputs?

Fortunately, Kani is able to report a coverage metric for each proof harness.
Try running:

```
cargo kani --visualize --harness verify_success
```

The beginning of the report includes coverage information.
Clicking through to the file will show fully-covered lines in green.
Lines not covered by our proof harness will show in red.

Try changing the assumption in the proof harness to `x < 2048`.
Now the harness won't be testing all possible cases.
Rerun `cargo kani --visualize`.
Look at the report: you'll see we no longer have 100% coverage of the function.

## Summary

In this section:

1. We saw Kani find panics, assertion failures, and even some other failures like unsafe dereferencing of null pointers.
2. We saw Kani find failures that testing could not easily find.
3. We saw how to write a proof harness and use `kani::any()`.
4. We saw how to get a failing **trace** using `kani --visualize`
5. We saw how proof harnesses are used to set up preconditions with `kani::assume()`.
6. We saw how to obtain **coverage** metrics and use them to ensure our proofs are covering as much as they should be.
