# Failures that RMC can spot

In the [last section](./tutorial-first-steps.md) we saw RMC spot two major kinds of failures: assertions and panics.
If the proof harness allows some program trace that results in a panic, then RMC will report that as a failure.
We additionally saw very briefly a couple of other kinds of failures, like null pointer dereferences and overflow.
In this section, we're going to expand on these additional checks, to give you an idea of what other problems RMC will find.

## Bounds checking and pointers

Rust is safe by default, and so includes dynamic (run-time) bounds checking where needed.
Consider this Rust code (which can be found under [`rmc-docs/src/tutorial/kinds-of-failure`](https://github.com/model-checking/rmc/tree/main/rmc-docs/src/tutorial/kinds-of-failure/)):

```rust
{{#include tutorial/kinds-of-failure/tests/bounds-check.rs:code}}
```

We can again write a simple property test against this code:

```rust
{{#include tutorial/kinds-of-failure/tests/bounds-check.rs:proptest}}
```

This property test will immediately find the failing case because of this dynamic check.

But what if we change this function to use unsafe Rust:

```rust
return unsafe { *a.get_unchecked(i % a.len() + 1) };
```

Now the error becomes invisible to this test:

```
# cargo test
[...]
test tests::doesnt_crash ... ok
```

But we're able to check this unsafe code with RMC:

```rust
{{#include tutorial/kinds-of-failure/tests/bounds-check.rs:rmc}}
```

```
# rmc tests/bounds-check.rs
[...]
** 1 of 468 failed (2 iterations)
VERIFICATION FAILED
```

Notice there were a *lot* of verification conditions being checked: in the above output, 468! (It may change for you.)
This is a result of using the standard library `Vec` implementation, which means our harness actually used quite a bit of code, short as it looks.
RMC is inserting a lot more checks than appear as asserts in our code, so the output can be large.
Let's narrow that output down a bit:

```
# rmc tests/bounds-check.rs | grep FAIL
[get_wrapped.pointer_dereference.5] line 10 dereference failure: pointer outside object bounds in *var_5: FAILURE
VERIFICATION FAILED
```

Notice that, for RMC, this has gone from a simple bounds-checking problem to a pointer-checking problem.
RMC will check operations on pointers to ensure they're not potentially invalid memory accesses.
Any unsafe code that manipulates pointers will, as we see here, raise failures if its behavior is actually unsafe. 

Consider trying a few more small exercises with this example:

1. Exercise: Switch back to the normal/safe indexing operation and re-try RMC. What changes compared to the unsafe operation and why?
(Try predicting the answer, then seeing if you got it right.)
2. Exercise: [Remember how to get a trace from RMC?](./tutorial-first-steps.md#getting-a-trace) Find out what inputs it failed on.
3. Exercise: Fix the error, run RMC, and see a successful verification.
4. Exercise: Try switching back to the unsafe code (now with the error fixed) and re-run RMC. It should still successfully verify.

<details>
<summary>Click to see explanation for exercise 1</summary>

Having switched back to the safe indexing operation, RMC reports two failures instead of just one:

```
# rmc tests/bounds-check.rs | grep FAIL
[get_wrapped.assertion.3] line 9 index out of bounds: the length is move _12 but the index is _5: FAILURE
[get_wrapped.pointer_dereference.5] line 9 dereference failure: pointer outside object bounds in a.data[var_5]: FAILURE
VERIFICATION FAILED
```

The first is Rust's implicit assertion for the safe indexing operation.
The second is RMC's check to ensure the pointer operation is actually safe.
This pattern (two checks for similar issues in safe Rust code) is common, and we'll see it again in the next section.

</details>

<details>
<summary>Click to see explanation for exercise 2</summary>

Having run `rmc --visualize` and clicked on one of the failures to see a trace, there are three things to immediately notice:

1. This trace is huge. The standard library `Vec` is involved, there's a lot going on.
2. The top of the trace file contains some "trace navigation tips" that might be helpful in navigating the trace.
3. There's a lot of generated code and it's really hard to just read the trace itself.

To navigate this trace to find the information you need, we recommend searching for things you expect to be somewhere in the trace:

1. Search the document for `rmc::nondet` or `variable_of_interest =` such as `size =`.
We can use this to find out what example values lead to a problem.
In this case, where we just have a couple of `rmc::nondet` values in our proof harness, we can learn a lot just by seeing what these are.
In this trace we find (and the values you get may be different):

```
Step 23: Function main, File tests/bounds-check.rs, Line 43
let size: usize = rmc::nondet();
size = 0ul

Step 27: Function main, File tests/bounds-check.rs, Line 45
let index: usize = rmc::nondet();
index = 0ul

Step 36: Function main, File tests/bounds-check.rs, Line 43
let size: usize = rmc::nondet();
size = 2464ul

Step 39: Function main, File tests/bounds-check.rs, Line 45
let index: usize = rmc::nondet();
index = 2463ul
```

Try not to be fooled by the first assignments: we're seeing zero-initialization there.
They get overridden by the later assignments.
You may see different values here, as it depends on the solver's behavior.

2. Try searching for "failure:". This will be near the end of the document.
Now you can try reverse-searching for assignments to the variables involved.
For example, search upwards from the failure for `i =`.

These two techniques should help you find both the nondeterministic inputs, and see what values were involved in the failing assertion.

</details>

## Overflow and math errors

Consider a different variant on the above function:

```rust
fn get_wrapped(i: usize, a: &[u32]) -> u32 {
    return a[i % a.len()];
}
```

We've corrected the out-of-bounds access, but now we've omitted the "base case": what to return on an empty list.
RMC will spot this not as a bound error, but as a mathematical error: on an empty list the modulus operator (`%`) will cause a division by zero.

1. Exercise: Try to run RMC on the above, to see what this kind of failure looks like.

Rust also performs runtime safety checks for integer overflows, much like it does for bounds checks.
Consider this code (from `tests/overflow.rs`):

```rust
{{#include tutorial/kinds-of-failure/tests/overflow.rs:code}}
```

A trivial function, but if we write a property test for it, we immediately find inputs where it fails, thanks to Rust's dynamic checks.
RMC will find these failures as well.
Here's the output from RMC:

```
# rmc tests/overflow.rs
[...]
** Results:
./tests/overflow.rs function simple_addition
[simple_addition.assertion.1] line 6 attempt to compute `move _3 + move _4`, which would overflow: FAILURE
[simple_addition.overflow.1] line 6 arithmetic overflow on unsigned + in var_3 + var_4: FAILURE

** 2 of 2 failed (2 iterations)
VERIFICATION FAILED
```

Notice the two failures: the Rust-inserted overflow check (`simple_addition.assertion.1`) and RMC's explicit overflow check (`simple_addition.overflow.1`).

> **NOTE:** You could attempt to fix this issue by using Rust's alternative mathematical functions with explicit overflow behavior.
For instance, instead of `a + b` write `a.wrapping_add(b)`.
>
> However, [at the present time](https://github.com/model-checking/rmc/issues/480), while this disables the dynamic assertion that Rust inserts, it does not disable the additional RMC overflow check.
> As a result, this currently still fails in RMC.

### Exercise: Classic overflow failure

One of the classic subtle bugs that persisted in many implementations for a very long time is finding the midpoint in quick sort.
This often naively looks like this (from `tests/overflow-quicksort.rs`):

```rust
{{#include tutorial/kinds-of-failure/tests/overflow-quicksort.rs:code}}
```

RMC immediately spots the bug in the above code.

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
Hence, the need to add an assumption that would make that impossible.
(Adding an assumption, though, means there's a new way to "use it wrong." Perhaps we'd like to avoid that!)

After that, you might wonder how to "prove your new implementation correct."
After all, what does "correct" even mean?
Often we're using a good approximation of correct, such as the equivalence of two implementations (often one much "simpler" than the other somehow).
Here's one possible assertion to make that obvious:

```rust
assert!(result as u64 == (a as u64 + b as u64) / 2);
```

Since this implementation is just the original one, but cast to a wider unsigned integer type, it should have the same result but without overflowing.
When RMC tells us both of these methods yield the same exact result, that gives us additional confidence that we haven't overlooked something.

</details>

## Future work

RMC notably does not currently check the following:

1. Concurrency bugs, deadlocks, or data races.
It's possible RMC may be extended in the future to find such issues.

2. Rust type invariants.
For example, it's undefined behavior in Rust to produce a value of type `bool` that isn't `0` or `1`.
RMC will not spot this error (in presumably unsafe code), yet.

3. Fully generic functions.
To write a proof harness and call functions, they must be fully "monomorphized."
This means we can't currently check a generic function (`foo<T>`) generically.
Proof harnesses have to be written specializing type parameters (`T`) to concrete types (e.g. `u32`), and check those instead.


## Summary

In this section:

1. We saw RMC spot potential bounds check errors.
2. We saw RMC spot actually-unsafe dereferencing of a raw pointer to invalid memory.
3. We saw RMC spot a division by zero error.
4. We saw RMC spot overflowing addition.
5. As an exercise, we tried proving an assertion (finding the midpoint) that was not completely trivial.
