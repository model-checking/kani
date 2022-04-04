# First steps with Kani

Kani is unlike the testing tools you may already be familiar with.
Much of testing is concerned with thinking of new corner cases that need to be covered.
With Kani, all the corner cases are covered from the start, and the new concern is narrowing down the scope to something manageable for the checker.

Consider this first program (which can be found under [`docs/src/tutorial/kani-first-steps`](https://github.com/model-checking/kani/tree/main/docs/src/tutorial/kani-first-steps/)):

```rust,noplaypen
{{#include tutorial/kani-first-steps/src/lib.rs:code}}
```

Think about the test harness you would need to write to test this function.
You would need figure out a whole set of arguments to call the function with that would exercise each branch.
You would need to keep that test harness up-to-date with the code, in case some of the branches change.
And if this function was more complicated—for example, if some of the branches depended on global state—the test harness would be even more onerous to write.

We can try to property test a function like this, but if we're naive about it (and consider all possible `u32` inputs), then it's unlikely we'll ever find the bug.

```rust,noplaypen
{{#include tutorial/kani-first-steps/src/lib.rs:proptest}}
```

```
# cargo test
[...]
test tests::doesnt_crash ... ok
```

There's only 1 in 4 billion inputs that fail, so it's vanishingly unlikely the property test will find it, even with a million samples.

With Kani, however:

```rust,noplaypen
{{#include tutorial/kani-first-steps/src/lib.rs:kani}}
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
Notably, we haven't had to write explicit assertions in our "proof harness": by default, Kani will find a host of erroneous conditions which include a reachable call to `panic` or a failing `assert`.

### Getting a trace

By default, Kani only reports failures, not how the failure happened.
This is because, in its full generality, understanding how a failure happened requires exploring a full (potentially large) execution trace.
Here, we've just got some nondeterministic inputs up front, but that's something of a special case that has a "simpler" explanation (just the choice of nondeterministic input).

To see traces, run:

```
kani --visualize src/lib.rs
open report/html/index.html
```

The first command runs Kani and generates the html-based report in `report/`.
The second command opens that report in your default browser (on mac, on linux desktops try `xdg-open`).
From this report, we can find the trace of the failure and filter through it to find the relevant line (at present time, an unfortunate amount of generated code is present in the trace):

```
let x: u32 = kani::any();
x = 1023u
```

Here we're seeing the line of code and the value assigned in this particular trace.
Like property testing, this is just one example of a failure.
To find more, we'd presumably fix this issue and then re-run Kani.

### Exercise: Try other failures

We put an explicit panic in this function, but it's not the only kind of failure Kani will find.
Try a few other types of errors.

For example, instead of panicking we could try explicitly dereferencing a null pointer:

```rust,noplaypen
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

Still, it is just a warning, and we can run the code without test failures just as before.
But Kani still catches the issue:

```
[...]
RESULTS:
[...]
Check 2: foo.pointer_dereference.1
         - Status: FAILURE
         - Description: "dereference failure: pointer NULL"
[...]
VERIFICATION:- FAILED
```

**Can you find an example where the Rust compiler will not complain, and Kani will?**

<details>
<summary>Click to show one possible answer</summary>

```
return 1 << x;
```

Overflow (addition, multiplication, etc, and this case, [bitshifting by too much](https://github.com/rust-lang/rust/issues/10183)) is also caught by Kani:

```
RESULTS:
[...]
Check 3: foo.assertion.1
         - Status: FAILURE
         - Description: "attempt to shift left with overflow"

Check 4: foo.undefined-shift.1
         - Status: FAILURE
         - Description: "shift distance too large"
[...]
VERIFICATION:- FAILED
```

</details>

## Assertions, Assumptions, and Harnesses

It seems a bit odd that we can take billions of inputs, but our function clearly only handles up to a few thousand.
Let's codify this fact about our function by asserting some reasonable bound on our input, after we've fixed our bug:

```rust,noplaypen
{{#include tutorial/kani-first-steps/src/final_form.rs:code}}
```

Now we have stated our previously implicit expectation: this function should never be called with inputs that are too big.
But if we attempt to verify this, we get a problem:

```
[...]
RESULTS:
[...]
Check 3: final_form::estimate_size.assertion.1
         - Status: FAILURE
         - Description: "assertion failed: x < 4096"
[...]
VERIFICATION:- FAILED
```

We intended this to be a precondition of calling the function, but Kani is treating it like a failure.
If we call this function with too large of a value, it will crash with an assertion failure.
But we know that, that was our intention.

This is the purpose of _proof harnesses_.
Much like property testing (which would also find this assertion failure as a bug), we need to set up our preconditions, call the function in question, then assert our post conditions.
Here's a revised example of the proof harness, one that now succeeds:

```rust,noplaypen
{{#include tutorial/kani-first-steps/src/final_form.rs:kani}}
```

But now we must wonder if we've really fully tested our function.
What if we revise the function, but forget to update the assumption in our proof harness to cover the new range of inputs?

Fortunately, Kani is able to report a coverage metric for each proof harness.
Try running:

```
kani --visualize src/final_form.rs --harness verify_success
open report/html/index.html
```

The beginning of the report includes coverage information.
Clicking through to the file will show fully-covered lines in green.
Lines not covered by our proof harness will show in red.

1. Try changing the assumption in the proof harness to `x < 2048`. Now the harness won't be testing all possible cases.
2. Rerun `kani --visualize` on the file
3. Look at the report: you'll see we no longer have 100% coverage of the function.


## Summary

In this section:

1. We saw Kani find panics, assertion failures, and even some other failures like unsafe dereferencing of null pointers.
2. We saw how to get a failing trace using `kani --visualize`
3. We saw how proof harnesses are used to set up preconditions and assert postconditions.
4. We saw how to obtain coverage metrics and use them to ensure our proofs are covering as much as they should be.
