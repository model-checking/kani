# Failures that Kani can spot

In the [last section](./tutorial-first-steps.md), we saw Kani spot two major kinds of failures: assertions and panics.
If the proof harness allows some program execution that results in a panic, then Kani will report that as a failure.
In addition, we saw (very briefly) a couple of other kinds of failures: null pointer dereferences and overflows.
In this section, we're going to expand on these additional checks, to give you an idea of what other problems Kani will find.

## Bounds checking and pointers

Rust is safe by default, and so includes dynamic (run-time) bounds checking where needed.
Consider this Rust code (available [here](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/kinds-of-failure/src/bounds_check.rs)):

```rust
{{#include tutorial/kinds-of-failure/src/bounds_check.rs:code}}
```

We can again write a simple property test against this code:

```rust
{{#include tutorial/kinds-of-failure/src/bounds_check.rs:proptest}}
```

This property test will immediately find a failing case, thanks to Rust's built-in bounds checking.

But what if we change this function to use unsafe Rust?

```rust
return unsafe { *a.get_unchecked(i % a.len() + 1) };
```

Now the error becomes invisible to this test:

```
# cargo test
[...]
test bounds_check::tests::doesnt_crash ... ok
```

The property test still causes an out-of-bounds access, but this undefined behavior does not necessarily cause an immediate crash.
(This is part of why undefined behavior is so difficult to debug.)
Through the use of unsafe code, we removed the runtime check for an out of bounds access.
It just turned out that none of the randomly generated tests triggered behavior that actually crashed.
But if we write a Kani proof harness:

```rust
{{#include tutorial/kinds-of-failure/src/bounds_check.rs:kani}}
```

And run this proof with:

```bash
cargo kani --harness bound_check
```

We still see a failure from Kani, even without Rust's runtime bounds checking.

> Also, notice there were many checks in the verification output.
> (At time of writing, 351.)
> This is a result of using the standard library `Vec` implementation, which means our harness actually used quite a bit of code, short as it looks.
> Kani is inserting a lot more checks than appear as asserts in our code, so the output can be large.

We get the following summary at the end:

```
SUMMARY: 
 ** 1 of 351 failed
Failed Checks: dereference failure: pointer outside object bounds
 File: "./src/bounds_check.rs", line 11, in bounds_check::get_wrapped

VERIFICATION:- FAILED
```

Notice that, for Kani, this has gone from a simple bounds-checking problem to a pointer-checking problem.
Kani will check operations on pointers to ensure they're not potentially invalid memory accesses.
Any unsafe code that manipulates pointers will, as we see here, raise failures if its behavior is actually a problem.

Consider trying a few more small exercises with this example:

1. Exercise: Switch back to the normal/safe indexing operation and re-try Kani.
How does Kani's output change, compared to the unsafe operation?
(Try predicting the answer, then seeing if you got it right.)
2. Exercise: [Remember how to get a trace from Kani?](./tutorial-first-steps.md#getting-a-trace) Find out what inputs it failed on.
3. Exercise: Fix the error, run Kani, and see a successful verification.
4. Exercise: Try switching back to the unsafe code (now with the error fixed) and re-run Kani. Does it still verify successfully?

<details>
<summary>Click to see explanation for exercise 1</summary>

Having switched back to the safe indexing operation, Kani reports a bounds check failure:

```
SUMMARY:
 ** 1 of 343 failed (8 unreachable)
Failed Checks: index out of bounds: the length is less than or equal to the given index
 File: "src/bounds_check.rs", line 11, in bounds_check::get_wrapped

VERIFICATION:- FAILED
```

</details>

<details>
<summary>Click to see explanation for exercise 2</summary>

Having run `cargo kani --harness bound_check --visualize --enable-unstable` and clicked on one of the failures to see a trace, there are three things to immediately notice:

1. This trace is huge. Because the standard library `Vec` is involved, there's a lot going on.
2. The top of the trace file contains some "trace navigation tips" that might be helpful in navigating the trace.
3. There's a lot of generated code and it's really hard to just read the trace itself.

To navigate this trace to find the information you need, we again recommend searching for things you expect to be somewhere in the trace:

1. Search the page for `kani::any` or `<variable_of_interest> =` such as `size =` or `let size`.
We can use this to find out what example values lead to a problem.
In this case, where we just have a couple of `kani::any` values in our proof harness, we can learn a lot just by seeing what these are.
In this trace we find (and the values you get may be different):

```
Step 523: Function bound_check, File src/bounds_check.rs, Line 37
<- kani::any::<usize>
Step 524: Function bound_check, File src/bounds_check.rs, Line 37
size = 1ul (00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000001)
...
Step 537: Function bound_check, File src/bounds_check.rs, Line 39
<- kani::any::<usize>
Step 538: Function bound_check, File src/bounds_check.rs, Line 39
index = 18446744073709551615ul (11111111 11111111 11111111 11111111 11111111 11111111 11111111 11111111)
```

You may see different values here, as it depends on the solver's behavior.

2. Try searching for `failure:`. This will be near the end of the page.
You can now search upwards from a failure to see what values certain variables had.
Sometimes it can be helpful to change the source code to add intermediate variables, so their value is visible in the trace.
For instance, you might want to compute the index before indexing into the array.
That way you'd see in the trace exactly what value is being used.

These two techniques should help you find both the nondeterministic inputs, and the values that were involved in the failing assertion.

</details>

## Overflow and math errors

Consider a different variant on the function above:

```rust
fn get_wrapped(i: usize, a: &[u32]) -> u32 {
    return a[i % a.len()];
}
```

We've corrected the out-of-bounds access, but now we've omitted the "base case": what to return on an empty list.
Kani will spot this not as a bound error, but as a mathematical error: on an empty list the modulus operator (`%`) will cause a division by zero.

1. Exercise: Try to run Kani on this version of `get_wrapped`, to see what this kind of failure looks like.

Rust can also perform runtime safety checks for integer overflows, much like it does for bounds checks.
([Though Rust disables this by default in `--release` mode, it can be re-enabled.](https://doc.rust-lang.org/reference/expressions/operator-expr.html#overflow))
Consider this code (available [here](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/kinds-of-failure/src/overflow.rs)):

```rust
{{#include tutorial/kinds-of-failure/src/overflow.rs:code}}
```

A trivial function, but if we write a property test for it, we immediately find inputs where it fails, thanks to Rust's dynamic checks.
Kani will find these failures as well.
Here's the output from Kani:

```
# cargo kani --harness add_overflow
[...]
SUMMARY: 
 ** 1 of 2 failed
Failed Checks: attempt to add with overflow
 File: "./src/overflow.rs", line 7, in overflow::simple_addition

VERIFICATION:- FAILED
```

This issue can be fixed using Rust's alternative mathematical functions with explicit overflow behavior.
For instance, if the wrapping behavior is intended, you can write `a.wrapping_add(b)` instead of `a + b`.
Kani will then report no issues.

### Exercise: Classic overflow failure

A classic example of a subtle bug that persisted in many implementations for a very long time is "finding the midpoint" in quick sort.
This often naively looks like this (code available [here](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/kinds-of-failure/src/overflow_quicksort.rs)):

```rust
{{#include tutorial/kinds-of-failure/src/overflow_quicksort.rs:code}}
```

```
cargo kani --harness midpoint_overflow
```

Kani immediately spots the bug in the above code.

1. Exercise: Fix this function so it no longer overflows.
(Hint: depending on which approach you take, you may need to add the assumption that `high > low` to your proof harness.
Don't add that right away, see what happens if you don't. Just keep it in mind.)
2. Exercise: Prove your new implementation actually finds the midpoint correctly by adding an assertion to the test harness.

<details>
<summary>Click to see solutions for these exercises</summary>

A very common approach for resolving the overflow issue looks like this:

```rust
return low + (high - low) / 2;
```

But if you naively try this (try it!), you'll find a new underflow error: `high - low` might result in a negative number, but has type `u32`.
Hence, the need to add the assumption we suggested above, to make that impossible.
(Adding an assumption, though, means there's a new way to "use it wrong." Perhaps we'd like to avoid that! Can you avoid the assumption?)

After that, you might wonder how to "prove your new implementation correct."
After all, what does "correct" even mean?
Often we're using a good approximation of correct, such as the equivalence of two implementations (often one much "simpler" than the other somehow).
Here's one possible assertion we could write in the proof harness:

```rust
assert!(result as u64 == (a as u64 + b as u64) / 2);
```

You might have even come up with this approach to avoiding the overflow issue in the first place!
Having two different implementations, using different approaches, but proven to yield the same results, gives us greater confidence that we compute the correct result.

</details>

## Failures that Kani cannot spot

Check out [Limitations](./limitations.md) for information on the checks that Kani does not perform.
Notably, Kani is not prioritizing all Rust-specific notions of undefined behavior.

## Summary

In this section:

1. We saw Kani spot out-of-bounds accesses.
2. We saw Kani spot actually-unsafe dereferencing of a raw pointer to invalid memory.
3. We got more experience reading the traces that Kani generates, to debug a failing proof harness.
3. We saw Kani spot a division by zero error and an overflowing addition.
5. As an exercise, we tried proving an assertion (finding the midpoint) that was not completely trivial.
