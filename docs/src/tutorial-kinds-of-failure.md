# Failures that Kani can spot

In the [last section](./tutorial-first-steps.md) we saw Kani spot two major kinds of failures: Assertions and panics.
If the proof harness allows some program execution that results in a panic, then Kani will report that as a failure.
In addition, we saw very briefly a couple of other kinds of failures: Null pointer dereferences and overflows.
In this section, we're going to expand on these additional checks, to give you an idea of what other problems Kani will find.

## Bounds checking and pointers

Rust is safe by default, and so includes dynamic (run-time) bounds checking where needed.
Consider this Rust code (from [`src/bounds_check.rs`](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/kinds-of-failure/src/bounds_check.rs)):

```rust,noplaypen
{{#include tutorial/kinds-of-failure/src/bounds_check.rs:code}}
```

We can again write a simple property test against this code:

```rust,noplaypen
{{#include tutorial/kinds-of-failure/src/bounds_check.rs:proptest}}
```

This property test will immediately find the failing case because of this dynamic check.

But what if we change this function to use unsafe Rust?

```rust,noplaypen
return unsafe { *a.get_unchecked(i % a.len() + 1) };
```

Now the error becomes invisible to this test:

```
# cargo test
[...]
test bounds_check::tests::doesnt_crash ... ok
```

But we're able to check this unsafe code with Kani:

```rust,noplaypen
{{#include tutorial/kinds-of-failure/src/bounds_check.rs:kani}}
```

```
# kani src/bounds_check.rs --harness bound_check
[...]
SUMMARY:
 ** 1 of 459 failed
[...]
VERIFICATION:- FAILED
```

Notice there were a *lot* of verification conditions being checked in the above output: 459! (This number may change depending on the version.)
This is a result of using the standard library `Vec` implementation, which means our harness actually used quite a bit of code, short as it looks.
Kani is inserting a lot more checks than appear as asserts in our code, so the output can be large.
Let's narrow that output down a bit:

```
# kani src/bounds_check.rs --harness bound_check | grep Failed
Failed Checks: dereference failure: pointer outside object bounds
```

Notice that, for Kani, this has gone from a simple bounds-checking problem to a pointer-checking problem.
Kani will check operations on pointers to ensure they're not potentially invalid memory accesses.
Any unsafe code that manipulates pointers will, as we see here, raise failures if its behavior is actually unsafe. 

Consider trying a few more small exercises with this example:

1. Exercise: Switch back to the normal/safe indexing operation and re-try Kani. What changes compared to the unsafe operation and why?
(Try predicting the answer, then seeing if you got it right.)
2. Exercise: [Remember how to get a trace from Kani?](./tutorial-first-steps.md#getting-a-trace) Find out what inputs it failed on.
3. Exercise: Fix the error, run Kani, and see a successful verification.
4. Exercise: Try switching back to the unsafe code (now with the error fixed) and re-run Kani. It should still successfully verify.

<details>
<summary>Click to see explanation for exercise 1</summary>

Having switched back to the safe indexing operation, Kani reports two failures:

```
# kani src/bounds_check.rs --harness bound_check | grep Failed
Failed Checks: index out of bounds: the length is less than or equal to the given index
Failed Checks: dereference failure: pointer outside object bounds
```

The first is Rust's implicit assertion for the safe indexing operation.
The second is Kani's check to ensure the pointer operation is actually safe.
This pattern (two checks for similar issues in safe Rust code) is common, and we'll see it again in the next section.

</details>

<details>
<summary>Click to see explanation for exercise 2</summary>

Having run `kani --visualize` and clicked on one of the failures to see a trace, there are three things to immediately notice:

1. This trace is huge. Because the standard library `Vec` is involved, there's a lot going on.
2. The top of the trace file contains some "trace navigation tips" that might be helpful in navigating the trace.
3. There's a lot of generated code and it's really hard to just read the trace itself.

To navigate this trace to find the information you need, we recommend searching for things you expect to be somewhere in the trace:

1. Search the document for `kani::any` or `<variable_of_interest> =` such as `size =`.
We can use this to find out what example values lead to a problem.
In this case, where we just have a couple of `kani::any` values in our proof harness, we can learn a lot just by seeing what these are.
In this trace we find (and the values you get may be different):

```
Step 23: Function bound_check, File src/bounds_check.rs, Line 43
let size: usize = kani::any();
size = 0ul

Step 27: Function bound_check, File src/bounds_check.rs, Line 45
let index: usize = kani::any();
index = 0ul

Step 36: Function bound_check, File src/bounds_check.rs, Line 43
let size: usize = kani::any();
size = 2464ul

Step 39: Function main, File src/bounds_check.rs, Line 45
let index: usize = kani::any();
index = 2463ul
```

Try not to be fooled by the first assignments: we're seeing zero-initialization there.
Their values are overwritten by the later assignments.
You may see different values here, as it depends on the solver's behavior.

2. Try searching for "failure:". This will be near the end of the document.
Now you can try reverse-searching for assignments to the variables involved.
For example, search upwards from the failure for `i =`.

These two techniques should help you find both the nondeterministic inputs, and the values that were involved in the failing assertion.

</details>

## Overflow and math errors

Consider a different variant on the function above:

```rust,noplaypen
fn get_wrapped(i: usize, a: &[u32]) -> u32 {
    return a[i % a.len()];
}
```

We've corrected the out-of-bounds access, but now we've omitted the "base case": what to return on an empty list.
Kani will spot this not as a bound error, but as a mathematical error: on an empty list the modulus operator (`%`) will cause a division by zero.

1. Exercise: Try to run Kani on the above, to see what this kind of failure looks like.

Rust also performs runtime safety checks for integer overflows, much like it does for bounds checks.
Consider this code (from [`src/overflow.rs`](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/kinds-of-failure/src/overflow.rs)):

```rust,noplaypen
{{#include tutorial/kinds-of-failure/src/overflow.rs:code}}
```

A trivial function, but if we write a property test for it, we immediately find inputs where it fails, thanks to Rust's dynamic checks.
Kani will find these failures as well.
Here's the output from Kani:

```
# kani src/overflow.rs --harness add_overflow
[...]
RESULTS:
Check 1: simple_addition.assertion.1
         - Status: FAILURE
         - Description: "attempt to add with overflow"
[...]
VERIFICATION:- FAILED
```

This issue can be fixed using Rust's alternative mathematical functions with explicit overflow behavior.
For instance, instead of `a + b` write `a.wrapping_add(b)`.

### Exercise: Classic overflow failure

One of the classic subtle bugs that persisted in many implementations for a very long time is finding the midpoint in quick sort.
This often naively looks like this (from [`src/overflow_quicksort.rs`](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/kinds-of-failure/src/overflow_quicksort.rs)):

```rust,noplaypen
{{#include tutorial/kinds-of-failure/src/overflow_quicksort.rs:code}}
```

Kani immediately spots the bug in the above code.

1. Exercise: Fix this function so it no longer overflows.
(Hint: depending on which approach you take, you may need to add the assumption that `high > low` to your proof harness.
Don't add that right away, see what happens if you don't. Just keep it in mind.)
2. Exercise: Prove your new implementation actually finds the midpoint correctly by adding an assertion to the test harness.

<details>
<summary>Click to see solutions for these exercises</summary>

A very common approach for resolving the overflow issue looks like this:

```rust,noplaypen
return low + (high - low) / 2;
```

But if you naively try this (try it!), you'll find a new underflow error: `high - low` might result in a negative number, but has type `u32`.
Hence, the need to add an assumption that would make that impossible.
(Adding an assumption, though, means there's a new way to "use it wrong." Perhaps we'd like to avoid that!)

After that, you might wonder how to "prove your new implementation correct."
After all, what does "correct" even mean?
Often we're using a good approximation of correct, such as the equivalence of two implementations (often one much "simpler" than the other somehow).
Here's one possible assertion to make that obvious:

```rust,noplaypen
assert!(result as u64 == (a as u64 + b as u64) / 2);
```

Since this implementation is just the original one, but cast to a wider unsigned integer type, it should have the same result but without overflowing.
When Kani tells us both of these methods yield the same exact result, that gives us additional confidence that we haven't overlooked something.

</details>
